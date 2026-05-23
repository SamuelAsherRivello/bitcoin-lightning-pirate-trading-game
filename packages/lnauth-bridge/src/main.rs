use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

use bech32::{ToBase32, Variant};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1};
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use url::form_urlencoded;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 37374;
const SESSION_TTL_MINUTES: i64 = 5;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
enum AuthAction {
    Login,
    Register,
    Link,
    Auth,
}

impl AuthAction {
    fn as_lnurl_action(&self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Register => "register",
            Self::Link => "link",
            Self::Auth => "auth",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct BeginSessionRequest {
    action: AuthAction,
    callback_base_url: String,
}

#[derive(Clone, Debug, Serialize)]
struct SessionResponse {
    session_id: String,
    challenge_id: String,
    lnurl: String,
    qr_payload: String,
    action: AuthAction,
    status: BridgeSessionStatus,
    expires_at: DateTime<Utc>,
    linking_key_fingerprint: Option<String>,
    failure_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum BridgeSessionStatus {
    Created,
    Approved,
    Expired,
    Failed,
}

#[derive(Clone, Debug)]
struct BridgeSession {
    response: SessionResponse,
    k1_hex: String,
}

type Sessions = Arc<Mutex<HashMap<String, BridgeSession>>>;

fn main() {
    let bind_address =
        env::var("LNAUTH_BRIDGE_ADDRESS").unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_string());
    let port = env::var("LNAUTH_BRIDGE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let listen = format!("{bind_address}:{port}");

    let server = Server::http(&listen).unwrap_or_else(|error| {
        panic!("Could not start LNAuth bridge at http://{listen}: {error}");
    });
    let sessions = Arc::new(Mutex::new(HashMap::new()));

    println!("LNAuth bridge listening at http://{listen}");
    println!("Use the same LAN address in the Dioxus QR when scanning from a phone.");

    for request in server.incoming_requests() {
        let sessions = sessions.clone();
        handle_request(request, sessions);
    }
}

fn handle_request(mut request: Request, sessions: Sessions) {
    let method = request.method().clone();
    let url = request.url().to_string();

    if method == Method::Options {
        let _ = request.respond(empty_response(StatusCode(204)));
        return;
    }

    let result = if method == Method::Get && url == "/api/lnauth/health" {
        Ok(json_response(
            StatusCode(200),
            &serde_json::json!({ "ok": true }),
        ))
    } else if method == Method::Post && url == "/api/lnauth/session" {
        let mut body = String::new();
        request
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|error| format!("Could not read request body: {error}"))
            .and_then(|_| begin_session(&sessions, &body))
    } else if method == Method::Get && url.starts_with("/api/lnauth/session/") {
        let session_id = url.trim_start_matches("/api/lnauth/session/");
        get_session(&sessions, session_id)
    } else if method == Method::Get && url.starts_with("/api/lnauth/callback?") {
        approve_session(&sessions, &url)
    } else {
        Ok(text_response(StatusCode(404), "not found"))
    };

    let response = match result {
        Ok(response) => response,
        Err(error) => json_response(
            StatusCode(400),
            &serde_json::json!({
                "status": "ERROR",
                "reason": error,
            }),
        ),
    };

    let _ = request.respond(response);
}

fn begin_session(
    sessions: &Sessions,
    body: &str,
) -> Result<Response<std::io::Cursor<Vec<u8>>>, String> {
    let request: BeginSessionRequest =
        serde_json::from_str(body).map_err(|error| format!("Invalid JSON: {error}"))?;
    let callback_base_url = request.callback_base_url.trim().trim_end_matches('/');
    if !callback_base_url.starts_with("http://") && !callback_base_url.starts_with("https://") {
        return Err("callback_base_url must start with http:// or https://".to_string());
    }

    let mut k1_bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut k1_bytes);
    let k1_hex = hex::encode(k1_bytes);
    let now = Utc::now();
    let suffix = now.timestamp_millis();
    let session_id = format!("player-auth-{suffix}");
    let callback_url = format!(
        "{callback_base_url}/api/lnauth/callback?tag=login&k1={k1_hex}&action={}",
        request.action.as_lnurl_action()
    );
    let lnurl = bech32::encode(
        "lnurl",
        callback_url.as_bytes().to_base32(),
        Variant::Bech32,
    )
    .map_err(|error| format!("Could not encode LNURL: {error}"))?
    .to_uppercase();

    let response = SessionResponse {
        session_id: session_id.clone(),
        challenge_id: k1_hex.clone(),
        lnurl: callback_url,
        qr_payload: lnurl,
        action: request.action,
        status: BridgeSessionStatus::Created,
        expires_at: now + Duration::minutes(SESSION_TTL_MINUTES),
        linking_key_fingerprint: None,
        failure_reason: None,
    };

    let session = BridgeSession {
        response: response.clone(),
        k1_hex,
    };
    sessions
        .lock()
        .map_err(|_| "Session store is unavailable.".to_string())?
        .insert(session_id, session);

    Ok(json_response(StatusCode(201), &response))
}

fn get_session(
    sessions: &Sessions,
    session_id: &str,
) -> Result<Response<std::io::Cursor<Vec<u8>>>, String> {
    let mut sessions = sessions
        .lock()
        .map_err(|_| "Session store is unavailable.".to_string())?;
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| "Unknown LNAuth session.".to_string())?;
    expire_if_needed(session);
    Ok(json_response(StatusCode(200), &session.response))
}

fn approve_session(
    sessions: &Sessions,
    url: &str,
) -> Result<Response<std::io::Cursor<Vec<u8>>>, String> {
    let query = url
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or_else(|| "Missing callback query.".to_string())?;
    let params: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    if params.get("tag").map(String::as_str) != Some("login") {
        return Ok(wallet_error("Unsupported LNURL tag."));
    }

    let k1 = params
        .get("k1")
        .ok_or_else(|| "Missing k1.".to_string())?
        .to_ascii_lowercase();
    let key = params
        .get("key")
        .ok_or_else(|| "Missing key.".to_string())?
        .to_ascii_lowercase();
    let sig = params
        .get("sig")
        .ok_or_else(|| "Missing sig.".to_string())?
        .to_ascii_lowercase();

    let mut sessions = sessions
        .lock()
        .map_err(|_| "Session store is unavailable.".to_string())?;
    let session = sessions
        .values_mut()
        .find(|session| session.k1_hex == k1)
        .ok_or_else(|| "Unknown k1.".to_string())?;

    expire_if_needed(session);
    if session.response.status == BridgeSessionStatus::Expired {
        return Ok(wallet_error("LNAuth session expired."));
    }

    match verify_lnurl_auth_signature(&k1, &key, &sig) {
        Ok(()) => {
            session.response.status = BridgeSessionStatus::Approved;
            session.response.linking_key_fingerprint = Some(key);
            session.response.failure_reason = None;
            Ok(wallet_ok())
        }
        Err(error) => {
            session.response.status = BridgeSessionStatus::Failed;
            session.response.failure_reason = Some(error.clone());
            Ok(wallet_error(&error))
        }
    }
}

fn expire_if_needed(session: &mut BridgeSession) {
    if session.response.status == BridgeSessionStatus::Created
        && Utc::now() > session.response.expires_at
    {
        session.response.status = BridgeSessionStatus::Expired;
        session.response.failure_reason = Some("LNAuth session expired.".to_string());
    }
}

fn verify_lnurl_auth_signature(k1_hex: &str, key_hex: &str, sig_hex: &str) -> Result<(), String> {
    let k1_bytes = hex::decode(k1_hex).map_err(|_| "Invalid k1 hex.".to_string())?;
    if k1_bytes.len() != 32 {
        return Err("Invalid k1 length.".to_string());
    }

    let public_key_bytes = hex::decode(key_hex).map_err(|_| "Invalid key hex.".to_string())?;
    let signature_bytes = hex::decode(sig_hex).map_err(|_| "Invalid signature hex.".to_string())?;
    let public_key =
        PublicKey::from_slice(&public_key_bytes).map_err(|_| "Invalid public key.".to_string())?;
    let signature =
        Signature::from_der(&signature_bytes).map_err(|_| "Invalid DER signature.".to_string())?;
    let message = Message::from_digest_slice(&k1_bytes)
        .map_err(|_| "Invalid LNAuth challenge.".to_string())?;

    Secp256k1::verification_only()
        .verify_ecdsa(&message, &signature, &public_key)
        .map_err(|_| "Wallet signature did not verify.".to_string())
}

fn wallet_ok() -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(StatusCode(200), &serde_json::json!({ "status": "OK" }))
}

fn wallet_error(reason: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        StatusCode(200),
        &serde_json::json!({
            "status": "ERROR",
            "reason": reason,
        }),
    )
}

fn json_response<T: Serialize>(
    status: StatusCode,
    value: &T,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    with_common_headers(
        Response::from_data(body).with_status_code(status),
        "application/json",
    )
}

fn text_response(status: StatusCode, value: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    with_common_headers(
        Response::from_string(value).with_status_code(status),
        "text/plain",
    )
}

fn empty_response(status: StatusCode) -> Response<std::io::Cursor<Vec<u8>>> {
    with_common_headers(
        Response::from_data(Vec::new()).with_status_code(status),
        "text/plain",
    )
}

fn with_common_headers(
    mut response: Response<std::io::Cursor<Vec<u8>>>,
    content_type: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    for (name, value) in [
        ("Access-Control-Allow-Origin", "*"),
        ("Access-Control-Allow-Headers", "content-type"),
        ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        ("Cache-Control", "no-store"),
        ("Content-Type", content_type),
    ] {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response.add_header(header);
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::SecretKey;

    #[test]
    fn verifies_lnurl_auth_signature_for_k1_key_and_der_signature() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[7_u8; 32]).expect("secret key");
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let k1_bytes = [3_u8; 32];
        let message = Message::from_digest_slice(&k1_bytes).expect("message");
        let signature = secp.sign_ecdsa(&message, &secret_key);

        let k1_hex = hex::encode(k1_bytes);
        let key_hex = hex::encode(public_key.serialize());
        let sig_hex = hex::encode(signature.serialize_der());

        assert!(verify_lnurl_auth_signature(&k1_hex, &key_hex, &sig_hex).is_ok());
    }

    #[test]
    fn rejects_lnurl_auth_signature_for_wrong_challenge() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[9_u8; 32]).expect("secret key");
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let signed_message = Message::from_digest_slice(&[4_u8; 32]).expect("message");
        let signature = secp.sign_ecdsa(&signed_message, &secret_key);

        let wrong_k1_hex = hex::encode([5_u8; 32]);
        let key_hex = hex::encode(public_key.serialize());
        let sig_hex = hex::encode(signature.serialize_der());

        assert!(verify_lnurl_auth_signature(&wrong_k1_hex, &key_hex, &sig_hex).is_err());
    }
}
