use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{Handle, Shared};

pub(super) fn router() -> Router<Handle> {
    Router::new()
        .route("/.well-known/oauth-authorization-server", get(metadata))
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/token", post(token))
        .route("/oauth/device", post(device))
        .route("/oauth/revoke", post(revoke))
}

async fn metadata(State(shared): State<Handle>) -> Json<Value> {
    let host = shared.lock().unwrap().host.clone();
    Json(json!({
        "issuer": host,
        "authorization_endpoint": format!("{host}/oauth/authorize"),
        "token_endpoint": format!("{host}/oauth/token"),
        "device_authorization_endpoint": format!("{host}/oauth/device"),
        "revocation_endpoint": format!("{host}/oauth/revoke"),
    }))
}

async fn authorize(
    State(shared): State<Handle>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let mut state = shared.lock().unwrap();
    assert_eq!(query["client_id"], "memory-cli");
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["resource"], format!("{}/api", state.host));
    assert!(query["redirect_uri"].starts_with("http://127.0.0.1:"));
    state.challenge = Some(query["code_challenge"].clone());
    let target = format!(
        "{}?code=the-code&state={}&iss={}",
        query["redirect_uri"],
        query["state"],
        state.host.replace(':', "%3A").replace('/', "%2F")
    );
    Redirect::to(&target).into_response()
}

fn issue(state: &mut Shared) -> Json<Value> {
    state.issued += 1;
    let access = format!("access-{}", state.issued);
    let refresh = format!("refresh-{}", state.issued);
    state.access.insert(access.clone());
    state.refresh.insert(refresh.clone());
    Json(json!({
        "access_token": access, "refresh_token": refresh, "token_type": "Bearer", "expires_in": 3600,
    }))
}

fn refused(code: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": code }))).into_response()
}

async fn token(
    State(shared): State<Handle>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let mut state = shared.lock().unwrap();
    assert_eq!(form["client_id"], "memory-cli");
    match form["grant_type"].as_str() {
        "authorization_code" => {
            let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes()));
            if form["code"] != "the-code" || state.challenge.as_deref() != Some(digest.as_str()) {
                return refused("invalid_grant");
            }
            issue(&mut state).into_response()
        }
        "refresh_token" => {
            state.refresh_calls += 1;
            if !state.refresh.remove(&form["refresh_token"]) {
                return refused("invalid_grant");
            }
            issue(&mut state).into_response()
        }
        "urn:ietf:params:oauth:grant-type:device_code" => {
            if state.device_pending > 0 {
                state.device_pending -= 1;
                return refused("authorization_pending");
            }
            issue(&mut state).into_response()
        }
        _ => refused("unsupported_grant_type"),
    }
}

async fn device(
    State(shared): State<Handle>,
    Form(form): Form<HashMap<String, String>>,
) -> Json<Value> {
    let mut state = shared.lock().unwrap();
    assert_eq!(form["client_id"], "memory-cli");
    state.device_pending = 2;
    Json(json!({
        "device_code": "device-code", "user_code": "BCDF-GHJK",
        "verification_uri": format!("{}/device", state.host),
        "verification_uri_complete": format!("{}/device?user_code=BCDF-GHJK", state.host),
        "expires_in": 600, "interval": 0,
    }))
}

async fn revoke(
    State(shared): State<Handle>,
    Form(form): Form<HashMap<String, String>>,
) -> StatusCode {
    shared.lock().unwrap().revoked.push(form["token"].clone());
    StatusCode::OK
}
