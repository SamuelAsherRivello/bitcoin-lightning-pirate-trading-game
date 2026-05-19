use serde_json::json;
use ui::client::models::PolarAutomationProfile;
use ui::client::services::polar_mcp_connector::{
    bridge_request_timeout_message, bridge_url, extract_mcp_error, format_mcp_error,
    is_local_connector_url, is_transient_bridge_request_error, missing_required_tools,
    redact_sensitive_log_text, sanitized_log_value, validate_local_profile,
    validate_required_tools, REQUIRED_POLAR_TOOLS,
};

#[test]
fn connector_bridge_url_joins_trimmed_base_and_path() {
    let profile = PolarAutomationProfile {
        bridge_url: "http://localhost:37373/".to_string(),
        network_id: String::new(),
        bitcoin_backend_name: "GAME_BITCOIN".to_string(),
    };

    assert_eq!(
        bridge_url(&profile, "/api/mcp/execute"),
        "http://localhost:37373/api/mcp/execute"
    );
}

#[test]
fn connector_local_url_validation_accepts_only_local_http_bridge() {
    assert!(is_local_connector_url("http://localhost:37373"));
    assert!(is_local_connector_url("http://127.0.0.1:37373/"));
    assert!(!is_local_connector_url("https://localhost:37373"));
    assert!(!is_local_connector_url("http://192.168.1.44:37373"));
    assert!(!is_local_connector_url("http://localhost:37373/path"));
}

#[test]
fn connector_rejects_non_local_profiles_before_requests() {
    let profile = PolarAutomationProfile {
        bridge_url: "http://192.168.1.44:37373".to_string(),
        network_id: String::new(),
        bitcoin_backend_name: "GAME_BITCOIN".to_string(),
    };

    let error = validate_local_profile(&profile).expect_err("non-local profile");

    assert!(error.contains("local http://localhost"));
}

#[test]
fn connector_extracts_mcp_error_shapes() {
    assert_eq!(
        extract_mcp_error(&json!({ "success": false, "error": { "message": "LND starting" } })),
        Some("LND starting".to_string())
    );
    assert_eq!(
        extract_mcp_error(&json!({ "success": false })),
        Some("Polar MCP tool returned success=false.".to_string())
    );
    assert_eq!(extract_mcp_error(&json!({ "success": true })), None);
}

#[test]
fn connector_formats_missing_helper_error_with_recovery() {
    let message = format_mcp_error(
        "list_networks",
        "ENOENT: no such file or directory, open 'C:\\Users\\user\\polar.json'",
    );

    assert!(message.contains("Polar MCP tool list_networks could not run"));
    assert!(message.contains("Restart Polar"));
}

#[test]
fn connector_redacts_sensitive_error_text() {
    assert_eq!(
        redact_sensitive_log_text("macaroon path C:\\secret\\admin.macaroon"),
        "[redacted sensitive Polar detail]"
    );
    assert_eq!(
        format_mcp_error("list_networks", "token abc123"),
        "Polar MCP tool list_networks failed: [redacted sensitive Polar detail]"
    );
}

#[test]
fn connector_sanitizes_sensitive_json_keys() {
    let value = json!({
        "ok": true,
        "macaroonPath": "C:\\secret\\admin.macaroon",
        "nested": {
            "tlsCert": "cert-body",
            "name": "Alice"
        }
    });
    let sanitized = sanitized_log_value(&value);

    assert_eq!(sanitized["macaroonPath"], "[redacted]");
    assert_eq!(sanitized["nested"]["tlsCert"], "[redacted]");
    assert_eq!(sanitized["nested"]["name"], "Alice");
}

#[test]
fn connector_timeout_and_retry_messages_are_classified() {
    assert_eq!(
        bridge_request_timeout_message("POST", "http://localhost:37373/api/mcp/execute"),
        "Polar bridge POST http://localhost:37373/api/mcp/execute timed out after 90 seconds."
    );
    assert!(is_transient_bridge_request_error(
        "Polar bridge request failed: connection refused"
    ));
    assert!(!is_transient_bridge_request_error(
        "Polar MCP tool deposit_funds failed: permission denied"
    ));
}

#[test]
fn connector_validates_required_tool_discovery() {
    assert!(validate_required_tools(REQUIRED_POLAR_TOOLS.iter().copied()).is_ok());

    let discovered = REQUIRED_POLAR_TOOLS
        .iter()
        .copied()
        .filter(|tool| *tool != "pay_invoice");

    let error = validate_required_tools(discovered).expect_err("missing pay_invoice");

    assert!(error.contains("pay_invoice"));
}

#[test]
fn connector_reports_all_missing_required_tools() {
    let missing = missing_required_tools(["list_networks", "start_node"]);

    assert!(missing.contains(&"pay_invoice"));
    assert!(missing.contains(&"create_network"));
    assert!(!missing.contains(&"list_networks"));
}
