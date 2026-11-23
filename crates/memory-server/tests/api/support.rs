use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use bytes::Bytes;
use std::sync::{Arc, Mutex, PoisonError};

use memory_server::{AppState, router};
use object_store::memory::InMemory;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RequestChallengeResponse, Url,
};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) const ORIGIN: &str = "http://localhost:8080";

#[derive(Debug)]
pub(crate) struct Reply {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Value,
}

pub(crate) struct Raw {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
}

impl Raw {
    pub(crate) fn header(&self, name: &str) -> &str {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    }
}

pub(crate) struct App {
    pub(crate) router: Router,
    pub(crate) pool: PgPool,
    pub(crate) state: AppState,
}

pub(crate) struct Browser {
    router: Router,
    cookie: Option<String>,
    authenticator: Mutex<SoftPasskey>,
    credential_ids: Mutex<Vec<Vec<u8>>>,
}

impl App {
    pub(crate) fn new(pool: PgPool) -> Self {
        let state = AppState::new(pool.clone(), Arc::new(InMemory::new()), 1024);
        Self {
            router: router(state.clone()),
            pool,
            state,
        }
    }

    pub(crate) async fn form(&self, path: &str, pairs: &[(&str, &str)]) -> TestResult<Reply> {
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(pairs)
            .finish();
        let request = Request::post(path)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))?;
        read(self.router.clone().oneshot(request).await?).await
    }

    pub(crate) async fn router_post_json(&self, path: &str, body: Value) -> TestResult<Reply> {
        let request = Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))?;
        read(self.router.clone().oneshot(request).await?).await
    }

    pub(crate) async fn plain_get(&self, path: &str) -> TestResult<Reply> {
        let request = Request::get(path).body(Body::empty())?;
        read(self.router.clone().oneshot(request).await?).await
    }

    pub(crate) fn browser(&self) -> Browser {
        Browser {
            router: self.router.clone(),
            cookie: None,
            authenticator: Mutex::new(SoftPasskey::new(true)),
            credential_ids: Mutex::new(Vec::new()),
        }
    }

    pub(crate) async fn admin(&self) -> TestResult<(Browser, String)> {
        let link = self.state.bootstrap_link().await?.ok_or("users exist")?;
        let token = link.split_once("token=").ok_or("no token")?.1.to_owned();
        let mut browser = self.browser();
        let recovery = browser.sign_up("bootstrap", &token, "admin").await?;
        Ok((browser, recovery))
    }

    pub(crate) async fn member(&self, admin: &Browser, name: &str) -> TestResult<Browser> {
        let body = admin.confirmed("create_invite", json!({})).await?;
        let invite = admin.post("/api/invites", body).await?;
        let token = invite.body["token"].as_str().ok_or("no token")?.to_owned();
        let mut browser = self.browser();
        browser.sign_up("register", &token, name).await?;
        Ok(browser)
    }
}

pub(crate) async fn read(response: axum::response::Response) -> TestResult<Reply> {
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Ok(Reply {
        status,
        headers,
        body,
    })
}

impl Browser {
    pub(crate) fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method.clone())
            .uri(path)
            .header("origin", ORIGIN);
        if let Some(cookie) = &self.cookie {
            builder = builder.header("cookie", cookie);
        }
        if method == Method::GET {
            return builder.body(Body::empty()).unwrap_or_default();
        }
        builder = builder.header("content-type", "application/json");
        let text = body.map_or_else(|| "{}".to_owned(), Value::to_string);
        builder.body(Body::from(text)).unwrap_or_default()
    }

    pub(crate) async fn send(&self, request: Request<Body>) -> TestResult<Reply> {
        read(self.router.clone().oneshot(request).await?).await
    }

    pub(crate) async fn get(&self, path: &str) -> TestResult<Reply> {
        self.send(self.request(Method::GET, path, None)).await
    }

    pub(crate) async fn post(&self, path: &str, body: Value) -> TestResult<Reply> {
        self.send(self.request(Method::POST, path, Some(&body)))
            .await
    }

    pub(crate) async fn patch(&self, path: &str, body: Value) -> TestResult<Reply> {
        self.send(self.request(Method::PATCH, path, Some(&body)))
            .await
    }

    pub(crate) async fn exchange(
        &self,
        method: Method,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<Value>,
    ) -> TestResult<Reply> {
        let mut request = self.request(method, path, body.as_ref());
        for (name, value) in headers {
            request
                .headers_mut()
                .insert(axum::http::HeaderName::try_from(*name)?, value.parse()?);
        }
        self.send(request).await
    }

    pub(crate) async fn raw(
        &self,
        method: Method,
        path: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> TestResult<Raw> {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("origin", ORIGIN);
        if let Some(cookie) = &self.cookie {
            builder = builder.header("cookie", cookie);
        }
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let response = self
            .router
            .clone()
            .oneshot(builder.body(Body::from(body))?)
            .await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX).await?;
        Ok(Raw {
            status,
            headers,
            body,
        })
    }

    pub(crate) async fn delete(&self, path: &str) -> TestResult<Reply> {
        self.send(self.request(Method::DELETE, path, None)).await
    }

    pub(crate) fn clone_cookie(&self) -> String {
        self.cookie.clone().unwrap_or_default()
    }

    pub(crate) async fn get_with_cookie(&self, path: &str, cookie: &str) -> TestResult<Reply> {
        let request = Request::get(path)
            .header("cookie", cookie)
            .header("origin", ORIGIN);
        self.send(request.body(Body::empty())?).await
    }

    pub(crate) fn adopt_cookie(&mut self, reply: &Reply) {
        if let Some(value) = reply
            .headers
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
        {
            let pair = value.split(';').next().unwrap_or_default();
            self.cookie = Some(pair.to_owned());
        }
    }

    pub(crate) fn register_credential(&mut self, options: &Value) -> TestResult<Value> {
        let selection = &options["publicKey"]["authenticatorSelection"];
        assert_eq!(selection["residentKey"], "required");
        assert_eq!(selection["requireResidentKey"], true);
        let mut options = options.clone();
        // SoftPasskey cannot store resident keys, so it gets the request without the requirement.
        options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = json!(false);
        options["publicKey"]["authenticatorSelection"]["residentKey"] = json!("discouraged");
        let options: CreationChallengeResponse = serde_json::from_value(options)?;
        let credential = self
            .authenticator
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .do_registration(Url::parse(ORIGIN)?, options)?;
        self.credential_ids
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(credential.raw_id.clone());
        let mut credential = serde_json::to_value(credential)?;
        credential["response"]["transports"] = json!(["internal", "hybrid"]);
        Ok(credential)
    }

    pub(crate) fn assert_credential(&self, options: &Value) -> TestResult<PublicKeyCredential> {
        let options: RequestChallengeResponse = serde_json::from_value(options.clone())?;
        Ok(self
            .authenticator
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .do_authentication(Url::parse(ORIGIN)?, options)?)
    }

    pub(crate) fn assert_discoverable(
        &self,
        options: &Value,
        user_handle: &[u8],
    ) -> TestResult<PublicKeyCredential> {
        let ids = self
            .credential_ids
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let mut options = options.clone();
        // SoftPasskey cannot discover credentials, so the test names them; a real authenticator finds them itself.
        options["publicKey"]["allowCredentials"] = ids
            .iter()
            .map(|id| json!({"type": "public-key", "id": URL_SAFE_NO_PAD.encode(id)}))
            .collect();
        let mut credential = self.assert_credential(&options)?;
        credential.response.user_handle = Some(user_handle.to_vec());
        Ok(credential)
    }

    pub(crate) async fn confirm(&self, action: &str) -> TestResult<Value> {
        let start = self
            .post("/api/auth/confirm", json!({"action": action}))
            .await?;
        assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
        let credential = self.assert_credential(&start.body["options"])?;
        Ok(json!({"challenge": start.body["challenge"], "credential": credential}))
    }

    pub(crate) async fn confirmed(&self, action: &str, mut body: Value) -> TestResult<Value> {
        body["confirmation"] = self.confirm(action).await?;
        Ok(body)
    }

    pub(crate) async fn create_invite(&self) -> TestResult<Reply> {
        let body = self.confirmed("create_invite", json!({})).await?;
        self.post("/api/invites", body).await
    }

    pub(crate) async fn set_admin(&self, user_id: i64, is_admin: bool) -> TestResult<Reply> {
        let body = self
            .confirmed("set_admin", json!({"is_admin": is_admin}))
            .await?;
        self.patch(&format!("/api/users/{user_id}"), body).await
    }

    pub(crate) async fn delete_passkey(&self, id: i64) -> TestResult<Reply> {
        let body = self.confirmed("remove_passkey", json!({})).await?;
        self.exchange(
            Method::DELETE,
            &format!("/api/auth/passkeys/{id}"),
            &[],
            Some(body),
        )
        .await
    }

    pub(crate) async fn start_add_passkey(&self, name: &str) -> TestResult<Reply> {
        let body = self.confirmed("add_passkey", json!({"name": name})).await?;
        self.post("/api/auth/passkeys/start", body).await
    }

    pub(crate) async fn export(&self) -> TestResult<Raw> {
        let body = self.confirmed("export", json!({})).await?;
        self.raw(
            Method::POST,
            "/api/system/export",
            &[("content-type", "application/json")],
            body.to_string().into_bytes(),
        )
        .await
    }

    pub(crate) async fn sign_up(
        &mut self,
        flow: &str,
        token: &str,
        name: &str,
    ) -> TestResult<String> {
        let start = self
            .post(
                &format!("/api/auth/{flow}/start"),
                json!({"token": token, "username": name, "display_name": format!("Display {name}")}),
            )
            .await?;
        assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
        let credential = self.register_credential(&start.body["options"])?;
        let finish = self
            .post(
                &format!("/api/auth/{flow}/finish"),
                json!({"challenge": start.body["challenge"], "credential": credential}),
            )
            .await?;
        assert_eq!(finish.status, StatusCode::OK, "{:?}", finish.body);
        self.adopt_cookie(&finish);
        Ok(finish.body["recovery_code"]
            .as_str()
            .ok_or("no code")?
            .to_owned())
    }

    pub(crate) async fn login_discoverable(&mut self, user_handle: &[u8]) -> TestResult<Reply> {
        let start = self
            .post("/api/auth/login/discoverable/start", json!({}))
            .await?;
        assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
        let credential = self.assert_discoverable(&start.body["options"], user_handle)?;
        let finish = self
            .post(
                "/api/auth/login/discoverable/finish",
                json!({"challenge": start.body["challenge"], "credential": credential}),
            )
            .await?;
        if finish.status == StatusCode::OK {
            self.adopt_cookie(&finish);
        }
        Ok(finish)
    }

    pub(crate) async fn login(&mut self, name: &str) -> TestResult<Reply> {
        let start = self
            .post("/api/auth/login/start", json!({"username": name}))
            .await?;
        assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
        let credential = self.assert_credential(&start.body["options"])?;
        let finish = self
            .post(
                "/api/auth/login/finish",
                json!({"challenge": start.body["challenge"], "credential": credential}),
            )
            .await?;
        if finish.status == StatusCode::OK {
            self.adopt_cookie(&finish);
        }
        Ok(finish)
    }
}
