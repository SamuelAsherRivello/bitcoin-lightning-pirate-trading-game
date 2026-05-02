use chrono::{Duration, Utc};

use crate::client::models::{
    validate_nostr_username, NostrAuthorizationSession, NostrAuthorizationStatus, NostrIdentity,
    NostrIdentityStatus, NostrProfile, NostrProfileAction, NostrProfileError,
    NostrProfilePublishStatus, NostrProfileSource,
};
use crate::client::services::storage_service;

const MOCK_NOSTR_PUBLIC_KEY: &str =
    "0000000000000000000000000000000000000000000000000000000000000ace";
const MOCK_NOSTR_NPUB: &str = "npub1mockprofileidentity";
const SESSION_TTL_SECONDS: i64 = 30;

#[derive(Clone, Debug, PartialEq)]
pub struct GetNostrProfileSummaryRequest {
    pub preferred_relays: Vec<String>,
    pub allow_local_snapshot: bool,
    pub identity_public_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetNostrProfileSummaryResponse {
    pub identity: Option<NostrIdentity>,
    pub profile: Option<NostrProfile>,
    pub is_loading_from_relay: bool,
    pub status_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartNostrProfileAuthorizationRequest {
    pub action: NostrProfileAction,
    pub draft_username: Option<String>,
    pub identity_public_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartNostrProfileAuthorizationResponse {
    pub session: NostrAuthorizationSession,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubmitNostrProfileNameRequest {
    pub session: NostrAuthorizationSession,
    pub username: String,
    pub preferred_relays: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubmitNostrProfileNameResponse {
    pub identity: NostrIdentity,
    pub profile: NostrProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CancelNostrProfileEditRequest {
    pub session_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CancelNostrProfileEditResponse {
    pub profile_unchanged: bool,
}

pub async fn get_nostr_profile_summary(
    request: GetNostrProfileSummaryRequest,
) -> Result<GetNostrProfileSummaryResponse, String> {
    let profile = request
        .allow_local_snapshot
        .then(storage_service::load_nostr_profile_snapshot)
        .flatten()
        .filter(|profile| {
            request
                .identity_public_key
                .as_ref()
                .is_none_or(|public_key| profile.public_key == *public_key)
        });
    let identity = profile.as_ref().map(identity_for_profile);
    let status_message = profile
        .as_ref()
        .map(|_| "Using local Nostr profile snapshot.".to_string());

    Ok(GetNostrProfileSummaryResponse {
        identity,
        profile,
        is_loading_from_relay: false,
        status_message,
    })
}

pub async fn start_nostr_profile_authorization(
    request: StartNostrProfileAuthorizationRequest,
) -> Result<StartNostrProfileAuthorizationResponse, String> {
    let now = Utc::now();
    let action = match request.action {
        NostrProfileAction::Login => "login",
        NostrProfileAction::SetProfileName => "set-profile-name",
    };
    let session_id = format!("nostr-profile-{}", now.timestamp_millis());
    let draft = request.draft_username.unwrap_or_default();
    let qr_payload = format!("nostr+profile://{action}?session={session_id}&name={draft}");
    let public_key = request.identity_public_key;

    Ok(StartNostrProfileAuthorizationResponse {
        session: NostrAuthorizationSession {
            session_id,
            action: request.action,
            qr_payload,
            status: NostrAuthorizationStatus::Pending,
            public_key,
            expires_at: now + Duration::seconds(SESSION_TTL_SECONDS),
            last_error: None,
        },
    })
}

pub async fn submit_nostr_profile_name(
    request: SubmitNostrProfileNameRequest,
) -> Result<SubmitNostrProfileNameResponse, NostrProfileError> {
    let username = validate_nostr_username(&request.username)?;
    if request.session.expires_at <= Utc::now()
        || request.session.status == NostrAuthorizationStatus::Expired
    {
        return Err(NostrProfileError::AuthorizationExpired);
    }
    if !matches!(request.session.status, NostrAuthorizationStatus::Approved) {
        return Err(NostrProfileError::AuthorizationRequired);
    }

    let identity = NostrIdentity {
        public_key: request
            .session
            .public_key
            .unwrap_or_else(|| MOCK_NOSTR_PUBLIC_KEY.to_string()),
        npub: MOCK_NOSTR_NPUB.to_string(),
        status: NostrIdentityStatus::Authenticated,
        authenticated_at: Some(Utc::now()),
        last_error: None,
    };
    let profile =
        publish_mock_nostr_metadata(&identity, username, request.preferred_relays).await?;
    storage_service::save_nostr_profile_snapshot(&profile);

    Ok(SubmitNostrProfileNameResponse { identity, profile })
}

pub async fn cancel_nostr_profile_edit(
    _request: CancelNostrProfileEditRequest,
) -> Result<CancelNostrProfileEditResponse, String> {
    Ok(CancelNostrProfileEditResponse {
        profile_unchanged: true,
    })
}

async fn publish_mock_nostr_metadata(
    identity: &NostrIdentity,
    username: String,
    relay_urls: Vec<String>,
) -> Result<NostrProfile, NostrProfileError> {
    touch_nostr_sdk_boundary();

    if username.eq_ignore_ascii_case("publish-fail") {
        return Err(NostrProfileError::PublishFailed);
    }

    Ok(NostrProfile {
        public_key: identity.public_key.clone(),
        username: Some(username),
        source: NostrProfileSource::Mock,
        publish_status: NostrProfilePublishStatus::Published,
        updated_at: Some(Utc::now()),
        relay_urls,
        last_error: None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn touch_nostr_sdk_boundary() {
    let _sdk_type = std::any::type_name::<nostr_sdk::prelude::Client>();
}

#[cfg(target_arch = "wasm32")]
fn touch_nostr_sdk_boundary() {}

fn identity_for_profile(profile: &NostrProfile) -> NostrIdentity {
    NostrIdentity {
        public_key: profile.public_key.clone(),
        npub: MOCK_NOSTR_NPUB.to_string(),
        status: NostrIdentityStatus::Authenticated,
        authenticated_at: profile.updated_at,
        last_error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approved_session() -> NostrAuthorizationSession {
        NostrAuthorizationSession {
            session_id: "nostr-profile-test".to_string(),
            action: NostrProfileAction::SetProfileName,
            qr_payload: "nostr+profile://set-profile-name".to_string(),
            status: NostrAuthorizationStatus::Approved,
            public_key: Some("abcdef123456".to_string()),
            expires_at: Utc::now() + Duration::seconds(30),
            last_error: None,
        }
    }

    #[test]
    fn start_authorization_uses_nostr_profile_payload() {
        let response = futures::executor::block_on(start_nostr_profile_authorization(
            StartNostrProfileAuthorizationRequest {
                action: NostrProfileAction::SetProfileName,
                draft_username: Some("jack".to_string()),
                identity_public_key: Some("abcdef123456".to_string()),
            },
        ))
        .expect("authorization starts");

        assert!(response.session.qr_payload.starts_with("nostr+profile://"));
        assert_eq!(response.session.status, NostrAuthorizationStatus::Pending);
        assert_eq!(response.session.public_key.as_deref(), Some("abcdef123456"));
    }

    #[test]
    fn submit_profile_name_validates_and_preserves_identity_scope() {
        storage_service::clear_nostr_profile_snapshot();
        let response =
            futures::executor::block_on(submit_nostr_profile_name(SubmitNostrProfileNameRequest {
                session: approved_session(),
                username: " jack ".to_string(),
                preferred_relays: vec!["wss://relay.example".to_string()],
            }))
            .expect("profile saved");

        assert_eq!(response.identity.public_key, "abcdef123456");
        assert_eq!(response.profile.public_key, "abcdef123456");
        assert_eq!(response.profile.username.as_deref(), Some("jack"));
        storage_service::clear_nostr_profile_snapshot();
    }

    #[test]
    fn profile_summary_is_scoped_to_current_identity() {
        storage_service::clear_nostr_profile_snapshot();
        let profile = NostrProfile {
            public_key: "jack-key".to_string(),
            username: Some("jack".to_string()),
            source: NostrProfileSource::Mock,
            publish_status: NostrProfilePublishStatus::Published,
            updated_at: Some(Utc::now()),
            relay_urls: Vec::new(),
            last_error: None,
        };
        storage_service::save_nostr_profile_snapshot(&profile);

        let matching =
            futures::executor::block_on(get_nostr_profile_summary(GetNostrProfileSummaryRequest {
                preferred_relays: Vec::new(),
                allow_local_snapshot: true,
                identity_public_key: Some("jack-key".to_string()),
            }))
            .expect("matching summary");
        let switched =
            futures::executor::block_on(get_nostr_profile_summary(GetNostrProfileSummaryRequest {
                preferred_relays: Vec::new(),
                allow_local_snapshot: true,
                identity_public_key: Some("bob-key".to_string()),
            }))
            .expect("switched summary");

        assert_eq!(
            matching.profile.and_then(|profile| profile.username),
            Some("jack".to_string())
        );
        assert!(switched.profile.is_none());
        storage_service::clear_nostr_profile_snapshot();
    }

    #[test]
    fn submit_profile_name_rejects_expired_authorization() {
        let mut session = approved_session();
        session.expires_at = Utc::now() - Duration::seconds(1);

        let result =
            futures::executor::block_on(submit_nostr_profile_name(SubmitNostrProfileNameRequest {
                session,
                username: "jack".to_string(),
                preferred_relays: Vec::new(),
            }));

        assert_eq!(result, Err(NostrProfileError::AuthorizationExpired));
    }

    #[test]
    fn submit_profile_name_rejects_pending_authorization() {
        let mut session = approved_session();
        session.status = NostrAuthorizationStatus::Pending;

        let result =
            futures::executor::block_on(submit_nostr_profile_name(SubmitNostrProfileNameRequest {
                session,
                username: "jack".to_string(),
                preferred_relays: Vec::new(),
            }));

        assert_eq!(result, Err(NostrProfileError::AuthorizationRequired));
    }

    #[test]
    fn publish_failure_does_not_claim_profile_saved() {
        let result =
            futures::executor::block_on(submit_nostr_profile_name(SubmitNostrProfileNameRequest {
                session: approved_session(),
                username: "publish-fail".to_string(),
                preferred_relays: Vec::new(),
            }));

        assert_eq!(result, Err(NostrProfileError::PublishFailed));
    }
}
