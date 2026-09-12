//! Interface-adapter layer.
//!
//! Holds injected use cases, invokes them, and renders their result into a
//! transport-neutral [`CommandOutcome`]. It depends only on [`use_cases`]; entity
//! types it needs are reached through `use_cases`' re-exports, so it never takes
//! a direct dependency on the entities crate.

use use_cases::{GreetError, GreetUser, SalutationProvider, Username};

/// Rendered result of a CLI command, ready for a presenter to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Human-readable message to display.
    pub message: String,
    /// Whether the command completed successfully.
    pub success: bool,
}

/// Driver that renders the greeting use case into a [`CommandOutcome`].
pub struct GreetDriver {
    interactor: Box<dyn GreetingService>,
}

/// Authentication HTTP delivery adapter.
pub mod auth {
    use std::sync::Arc;

    use axum::{
        Json, Router,
        body::Bytes,
        extract::{DefaultBodyLimit, State},
        http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
        response::IntoResponse,
        routing::post,
    };
    use tokio::sync::Semaphore;
    use use_cases::auth::{
        CredentialParseError, IssuedAccessToken, LoginCommand, LoginError, LoginService,
        RegisterUserCommand, RegisterUserError, RegisterUserService,
    };

    const HTTP_MAX_BODY_BYTES: usize = 64 * 1024;
    /// Process-wide cap on concurrent Argon2 authentication jobs.
    ///
    /// Declared in `knowledge/conventions/environment-declaration.md`.
    /// Additional accepted requests wait (`acquire().await`) before
    /// `spawn_blocking`; no new HTTP status is introduced on saturation.
    const AUTH_BLOCKING_CONCURRENCY_LIMIT: usize = 8;
    static AUTH_BLOCKING_PERMITS: Semaphore = Semaphore::const_new(AUTH_BLOCKING_CONCURRENCY_LIMIT);

    /// Axum primary adapter for the authentication use cases.
    pub struct AuthHttpApi {
        register: Arc<dyn RegisterUserService>,
        login: Arc<dyn LoginService>,
    }

    type AuthServices = (Arc<dyn RegisterUserService>, Arc<dyn LoginService>);

    /// Validated registration request carried from the HTTP boundary to the use case.
    pub struct RegisterRequest {
        command: RegisterUserCommand,
    }

    impl RegisterRequest {
        fn from_payload(payload: RegisterRequestPayload) -> Result<Self, CredentialParseError> {
            let command = RegisterUserCommand::parse(&payload.username, &payload.password)?;
            Ok(Self { command })
        }

        /// Consumes the request and returns its validated registration command.
        #[must_use]
        pub fn into_command(self) -> RegisterUserCommand {
            self.command
        }
    }

    #[derive(serde::Deserialize)]
    struct RegisterRequestPayload {
        username: String,
        password: String,
    }

    /// Validated login request carried from the HTTP boundary to the use case.
    pub struct LoginRequest {
        command: LoginCommand,
    }

    impl LoginRequest {
        fn from_payload(payload: LoginRequestPayload) -> Result<Self, CredentialParseError> {
            let command = LoginCommand::parse(&payload.username, &payload.password)?;
            Ok(Self { command })
        }

        /// Consumes the request and returns its validated login command.
        #[must_use]
        pub fn into_command(self) -> LoginCommand {
            self.command
        }
    }

    #[derive(serde::Deserialize)]
    struct LoginRequestPayload {
        username: String,
        password: String,
    }

    /// Typed login response carrier; JSON wire encoding stays private.
    pub struct AccessTokenResponse {
        access_token: IssuedAccessToken,
    }

    impl AccessTokenResponse {
        /// Builds a response from an issued application access token.
        #[must_use]
        pub fn new(access_token: IssuedAccessToken) -> Self {
            Self { access_token }
        }

        fn into_wire(self) -> AccessTokenWire {
            AccessTokenWire { access_token: self.access_token.as_str().to_owned() }
        }
    }

    #[derive(serde::Serialize)]
    struct AccessTokenWire {
        access_token: String,
    }

    impl AuthHttpApi {
        /// Builds the HTTP adapter from the registration and login services.
        #[must_use]
        pub fn new(register: Arc<dyn RegisterUserService>, login: Arc<dyn LoginService>) -> Self {
            Self { register, login }
        }

        /// Returns the axum router for this auth HTTP adapter.
        pub fn router(&self) -> Router {
            Router::new()
                .route("/register", post(register_user))
                .route("/login", post(login_user))
                // Explicit request-body ceiling declared in environment-declaration.md.
                .layer(DefaultBodyLimit::max(HTTP_MAX_BODY_BYTES))
                .with_state((Arc::clone(&self.register), Arc::clone(&self.login)))
        }
    }

    /// Returns true when `raw` has no empty media-type parameter slots.
    ///
    /// The `mime` crate accepts a trailing empty parameter list (`application/json;`).
    /// Those forms are malformed for this boundary and must yield HTTP 415.
    fn media_type_params_well_formed(raw: &str) -> bool {
        let raw = raw.trim();
        !raw.ends_with(';') && raw.split(';').skip(1).all(|part| !part.trim().is_empty())
    }

    fn exact_application_json(headers: &HeaderMap) -> bool {
        let mut values = headers.get_all(CONTENT_TYPE).iter();
        let (Some(header), None) = (values.next(), values.next()) else {
            return false;
        };
        let Ok(raw) = header.to_str() else {
            return false;
        };
        if !media_type_params_well_formed(raw) {
            return false;
        }
        let Ok(parsed) = raw.trim().parse::<mime::Mime>() else {
            return false;
        };
        // Reject structured suffixes such as application/problem+json.
        if parsed.type_() != mime::APPLICATION
            || parsed.subtype() != mime::JSON
            || parsed.suffix().is_some()
        {
            return false;
        }
        let mut params = parsed.params();
        match (params.next(), params.next()) {
            (None, None) => true,
            (Some((name, value)), None) => name == mime::CHARSET && value == mime::UTF_8,
            _ => false,
        }
    }

    async fn register_user(
        State((register, _login)): State<AuthServices>,
        headers: HeaderMap,
        body: Bytes,
    ) -> StatusCode {
        if !exact_application_json(&headers) {
            return StatusCode::UNSUPPORTED_MEDIA_TYPE;
        }

        let payload: RegisterRequestPayload = match serde_json::from_slice(&body) {
            Ok(payload) => payload,
            Err(_) => return StatusCode::BAD_REQUEST,
        };

        let request = match RegisterRequest::from_payload(payload) {
            Ok(request) => request,
            Err(_) => return StatusCode::BAD_REQUEST,
        };

        let permit = match AUTH_BLOCKING_PERMITS.acquire().await {
            Ok(permit) => permit,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        };
        let result = tokio::task::spawn_blocking(move || {
            let result = register.execute(request.into_command());
            drop(permit);
            result
        })
        .await;

        match result {
            Ok(Ok(())) => StatusCode::CREATED,
            Ok(Err(RegisterUserError::UserAlreadyExists)) => StatusCode::CONFLICT,
            Ok(Err(_)) | Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    async fn login_user(
        State((_register, login)): State<AuthServices>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        if !exact_application_json(&headers) {
            return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
        }

        let payload: LoginRequestPayload = match serde_json::from_slice(&body) {
            Ok(payload) => payload,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };

        let request = match LoginRequest::from_payload(payload) {
            Ok(request) => request,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };

        let permit = match AUTH_BLOCKING_PERMITS.acquire().await {
            Ok(permit) => permit,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        let result = tokio::task::spawn_blocking(move || {
            let result = login.execute(request.into_command());
            drop(permit);
            result
        })
        .await;

        match result {
            Ok(Ok(token)) => {
                let response = AccessTokenResponse::new(token);
                (StatusCode::OK, Json(response.into_wire())).into_response()
            }
            Ok(Err(LoginError::UserNotFound | LoginError::InvalidCredentials)) => {
                StatusCode::UNAUTHORIZED.into_response()
            }
            Ok(Err(_)) | Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use std::sync::{Arc, Mutex};

        use axum::{
            body::Body,
            extract::Json,
            http::{Request, StatusCode},
            routing::post,
        };
        use tower::ServiceExt;
        use use_cases::OpaqueAccessToken;
        use use_cases::auth::{
            IssuedAccessToken, LoginCommand, LoginError, LoginService, RegisterUserCommand,
            RegisterUserError, RegisterUserService,
        };

        use super::*;

        struct NoopRegister;

        impl RegisterUserService for NoopRegister {
            fn execute(&self, _command: RegisterUserCommand) -> Result<(), RegisterUserError> {
                Ok(())
            }
        }

        struct NoopLogin;

        impl LoginService for NoopLogin {
            fn execute(&self, _command: LoginCommand) -> Result<IssuedAccessToken, LoginError> {
                Err(LoginError::UserNotFound)
            }
        }

        fn api() -> AuthHttpApi {
            AuthHttpApi::new(Arc::new(NoopRegister), Arc::new(NoopLogin))
        }

        struct RecordingRegister {
            received: Arc<Mutex<Vec<(String, String)>>>,
        }

        impl RegisterUserService for RecordingRegister {
            fn execute(&self, command: RegisterUserCommand) -> Result<(), RegisterUserError> {
                self.received.lock().unwrap().push((
                    command.username().as_str().to_owned(),
                    command.password().expose_secret().to_owned(),
                ));
                Ok(())
            }
        }

        #[test]
        fn test_register_request_into_command_preserves_validated_credentials() {
            let payload: RegisterRequestPayload =
                serde_json::from_str(r#"{"username":" ada ","password":" secret "}"#).unwrap();
            let command = RegisterRequest::from_payload(payload).unwrap().into_command();

            assert_eq!(command.username().as_str(), "ada");
            assert_eq!(command.password().expose_secret(), " secret ");
        }

        #[tokio::test]
        async fn test_auth_http_api_register_route_registers_user() {
            let received = Arc::new(Mutex::new(Vec::new()));
            let app = AuthHttpApi::new(
                Arc::new(RecordingRegister { received: Arc::clone(&received) }),
                Arc::new(NoopLogin),
            )
            .router();
            let request = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"ada","password":"correct horse"}"#))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::CREATED);
            assert_eq!(
                *received.lock().unwrap(),
                vec![("ada".to_owned(), "correct horse".to_owned())]
            );
        }

        #[tokio::test]
        async fn test_register_request_json_strings_preserve_normalization_sensitive_values() {
            let received = Arc::new(Mutex::new(Vec::new()));
            let app = AuthHttpApi::new(
                Arc::new(RecordingRegister { received: Arc::clone(&received) }),
                Arc::new(NoopLogin),
            )
            .router();

            // NFC vs NFD usernames must remain distinct after JSON decode into RegisterRequest.
            let nfc = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"café","password":"sécret"}"#))
                .unwrap();
            let nfd_payload = "{\"username\":\"cafe\\u0301\",\"password\":\"se\\u0301cret\"}";
            let nfd = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json")
                .body(Body::from(nfd_payload))
                .unwrap();

            assert_eq!(app.clone().oneshot(nfc).await.unwrap().status(), StatusCode::CREATED);
            assert_eq!(app.oneshot(nfd).await.unwrap().status(), StatusCode::CREATED);
            let values = received.lock().unwrap().clone();
            assert_eq!(values.len(), 2);
            let nfc = values.first().unwrap();
            let nfd = values.get(1).unwrap();
            assert_ne!(nfc.0, nfd.0);
            assert_eq!(nfc.0.chars().count(), 4);
            assert_eq!(nfd.0.chars().count(), 5);
            assert_ne!(nfc.1, nfd.1);
            assert_eq!(nfc.1.chars().count(), 6);
            assert_eq!(nfd.1.chars().count(), 7);
        }

        #[tokio::test]
        async fn test_auth_http_api_register_route_rejects_empty_password() {
            let request = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"ada","password":""}"#))
                .unwrap();

            let response = api().router().oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_auth_http_api_router_is_axum_router() {
            let router = api().router();
            let _typed: Router = router;
        }

        #[tokio::test]
        async fn test_auth_http_api_json_rejection_returns_expected_statuses() {
            let app = api().router();

            let invalid_utf8 = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json")
                .body(Body::from(vec![0xff]))
                .unwrap();
            let response = app.clone().oneshot(invalid_utf8).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);

            let malformed_json = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .unwrap();
            let response = app.clone().oneshot(malformed_json).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);

            let unsupported_media_type = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "text/plain")
                .body(Body::from("{}"))
                .unwrap();
            let response = app.clone().oneshot(unsupported_media_type).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

            let structured_suffix_json = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/problem+json")
                .body(Body::from(r#"{"username":"ada","password":"secret"}"#))
                .unwrap();
            let response = app.clone().oneshot(structured_suffix_json).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

            let duplicate_charset = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json; charset=utf-8; charset=utf-8")
                .body(Body::from(r#"{"username":"ada","password":"secret"}"#))
                .unwrap();
            let response = app.clone().oneshot(duplicate_charset).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

            let malformed_quoted_charset = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json; charset=\"utf-8")
                .body(Body::from(r#"{"username":"ada","password":"secret"}"#))
                .unwrap();
            let response = app.clone().oneshot(malformed_quoted_charset).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

            let whitespace_after_equals = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json; charset= utf-8")
                .body(Body::from(r#"{"username":"ada","password":"secret"}"#))
                .unwrap();
            let response = app.clone().oneshot(whitespace_after_equals).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

            let unknown_parameter = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json; foo=bar")
                .body(Body::from(r#"{"username":"ada","password":"secret"}"#))
                .unwrap();
            let response = app.clone().oneshot(unknown_parameter).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

            let trailing_empty_parameter = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json;")
                .body(Body::from(r#"{"username":"ada","password":"secret"}"#))
                .unwrap();
            let response = app.clone().oneshot(trailing_empty_parameter).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

            let trailing_empty_after_charset = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json; charset=utf-8;")
                .body(Body::from(r#"{"username":"ada","password":"secret"}"#))
                .unwrap();
            let response = app.oneshot(trailing_empty_after_charset).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        }

        #[tokio::test]
        async fn test_auth_http_api_register_accepts_quoted_charset_utf8() {
            let received = Arc::new(Mutex::new(Vec::new()));
            let app = AuthHttpApi::new(
                Arc::new(RecordingRegister { received: Arc::clone(&received) }),
                Arc::new(NoopLogin),
            )
            .router();
            let request = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json; charset=\"utf-8\"")
                .body(Body::from(r#"{"username":"ada","password":"correct horse"}"#))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::CREATED);
            assert_eq!(
                *received.lock().unwrap(),
                vec![("ada".to_owned(), "correct horse".to_owned())]
            );
        }

        struct RecordingLogin {
            received: Arc<Mutex<Vec<(String, String)>>>,
            response: Result<String, LoginError>,
        }

        impl LoginService for RecordingLogin {
            fn execute(&self, command: LoginCommand) -> Result<IssuedAccessToken, LoginError> {
                self.received.lock().unwrap().push((
                    command.username().as_str().to_owned(),
                    command.password().expose_secret().to_owned(),
                ));
                match self.response {
                    Ok(ref token) => {
                        let opaque = OpaqueAccessToken::new(token.clone()).unwrap();
                        Ok(IssuedAccessToken::from_domain(opaque))
                    }
                    Err(LoginError::UserNotFound) => Err(LoginError::UserNotFound),
                    Err(LoginError::InvalidCredentials) => Err(LoginError::InvalidCredentials),
                    Err(_) => Err(LoginError::UserNotFound),
                }
            }
        }

        struct ThreadRecordingRegister {
            executed_on: Arc<Mutex<Option<std::thread::ThreadId>>>,
        }

        impl RegisterUserService for ThreadRecordingRegister {
            fn execute(&self, _command: RegisterUserCommand) -> Result<(), RegisterUserError> {
                *self.executed_on.lock().unwrap() = Some(std::thread::current().id());
                Ok(())
            }
        }

        struct ThreadRecordingLogin {
            executed_on: Arc<Mutex<Option<std::thread::ThreadId>>>,
        }

        impl LoginService for ThreadRecordingLogin {
            fn execute(&self, _command: LoginCommand) -> Result<IssuedAccessToken, LoginError> {
                *self.executed_on.lock().unwrap() = Some(std::thread::current().id());
                let token = OpaqueAccessToken::new("blocking-token".to_owned()).unwrap();
                Ok(IssuedAccessToken::from_domain(token))
            }
        }

        #[test]
        fn test_login_request_into_command_preserves_validated_credentials() {
            let payload: LoginRequestPayload =
                serde_json::from_str(r#"{"username":" ada ","password":" secret "}"#).unwrap();
            let command = LoginRequest::from_payload(payload).unwrap().into_command();

            assert_eq!(command.username().as_str(), "ada");
            assert_eq!(command.password().expose_secret(), " secret ");
        }

        #[test]
        fn test_access_token_response_new_preserves_issued_token_for_wire() {
            let opaque = OpaqueAccessToken::new("opaque-token-value".to_owned()).unwrap();
            let issued = IssuedAccessToken::from_domain(opaque);
            let wire = AccessTokenResponse::new(issued).into_wire();
            assert_eq!(wire.access_token, "opaque-token-value");
        }

        #[tokio::test]
        async fn test_auth_http_api_login_route_returns_access_token() {
            let received = Arc::new(Mutex::new(Vec::new()));
            let app = AuthHttpApi::new(
                Arc::new(NoopRegister),
                Arc::new(RecordingLogin {
                    received: Arc::clone(&received),
                    response: Ok("issued-opaque-token".to_owned()),
                }),
            )
            .router();
            let request = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"ada","password":"correct horse"}"#))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(
                body.get("access_token").and_then(|v| v.as_str()),
                Some("issued-opaque-token")
            );
            assert_eq!(
                *received.lock().unwrap(),
                vec![("ada".to_owned(), "correct horse".to_owned())]
            );
        }

        #[tokio::test]
        async fn test_auth_http_api_authentication_runs_services_on_blocking_threads() {
            let runtime_thread = std::thread::current().id();
            let register_thread = Arc::new(Mutex::new(None));
            let login_thread = Arc::new(Mutex::new(None));
            let app = AuthHttpApi::new(
                Arc::new(ThreadRecordingRegister { executed_on: Arc::clone(&register_thread) }),
                Arc::new(ThreadRecordingLogin { executed_on: Arc::clone(&login_thread) }),
            )
            .router();

            let register = Request::builder()
                .method("POST")
                .uri("/register")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"ada","password":"correct horse"}"#))
                .unwrap();
            assert_eq!(app.clone().oneshot(register).await.unwrap().status(), StatusCode::CREATED);

            let login = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"ada","password":"correct horse"}"#))
                .unwrap();
            assert_eq!(app.oneshot(login).await.unwrap().status(), StatusCode::OK);

            let register_thread = register_thread.lock().unwrap().expect("register executed");
            assert_ne!(register_thread, runtime_thread);
            let login_thread = login_thread.lock().unwrap().expect("login executed");
            assert_ne!(login_thread, runtime_thread);
        }

        #[tokio::test]
        async fn test_login_request_json_strings_preserve_normalization_sensitive_values() {
            let received = Arc::new(Mutex::new(Vec::new()));
            let app = AuthHttpApi::new(
                Arc::new(NoopRegister),
                Arc::new(RecordingLogin {
                    received: Arc::clone(&received),
                    response: Ok("tok".to_owned()),
                }),
            )
            .router();

            let nfc = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"café","password":"sécret"}"#))
                .unwrap();
            let nfd_payload = "{\"username\":\"cafe\\u0301\",\"password\":\"se\\u0301cret\"}";
            let nfd = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(nfd_payload))
                .unwrap();

            assert_eq!(app.clone().oneshot(nfc).await.unwrap().status(), StatusCode::OK);
            assert_eq!(app.oneshot(nfd).await.unwrap().status(), StatusCode::OK);
            let values = received.lock().unwrap().clone();
            assert_eq!(values.len(), 2);
            let nfc = values.first().unwrap();
            let nfd = values.get(1).unwrap();
            assert_ne!(nfc.0, nfd.0);
            assert_eq!(nfc.0.chars().count(), 4);
            assert_eq!(nfd.0.chars().count(), 5);
            assert_ne!(nfc.1, nfd.1);
            assert_eq!(nfc.1.chars().count(), 6);
            assert_eq!(nfd.1.chars().count(), 7);
        }

        #[tokio::test]
        async fn test_auth_http_api_login_route_rejects_empty_password() {
            let request = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"ada","password":""}"#))
                .unwrap();

            let response = api().router().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_auth_http_api_login_route_maps_invalid_credentials_to_unauthorized() {
            let app = AuthHttpApi::new(
                Arc::new(NoopRegister),
                Arc::new(RecordingLogin {
                    received: Arc::new(Mutex::new(Vec::new())),
                    response: Err(LoginError::InvalidCredentials),
                }),
            )
            .router();
            let request = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"ada","password":"wrong"}"#))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        #[tokio::test]
        async fn test_auth_http_api_login_json_rejection_returns_expected_statuses() {
            let app = api().router();

            let invalid_utf8 = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(vec![0xff]))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(invalid_utf8).await.unwrap().status(),
                StatusCode::BAD_REQUEST
            );

            let unsupported = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "text/plain")
                .body(Body::from("{}"))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(unsupported).await.unwrap().status(),
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            );

            let malformed = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .unwrap();
            assert_eq!(app.oneshot(malformed).await.unwrap().status(), StatusCode::BAD_REQUEST);
        }

        #[tokio::test]
        async fn test_auth_http_api_json_strings_are_not_unicode_normalized() {
            use std::sync::{Arc, Mutex};

            let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let seen_for_handler = Arc::clone(&seen);
            async fn capture(
                axum::extract::State(seen): axum::extract::State<Arc<Mutex<Vec<String>>>>,
                Json(value): Json<serde_json::Value>,
            ) -> StatusCode {
                let text =
                    value.get("name").and_then(serde_json::Value::as_str).unwrap().to_owned();
                seen.lock().unwrap().push(text);
                StatusCode::NO_CONTENT
            }

            let app = Router::new().route("/json", post(capture)).with_state(seen_for_handler);

            // U+00E9 (NFC) vs e + combining acute (NFD) must remain distinct after JSON decode.
            let nfc = Request::builder()
                .method("POST")
                .uri("/json")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"café"}"#))
                .unwrap();
            let nfd_payload = "{\"name\":\"cafe\\u0301\"}";
            let nfd = Request::builder()
                .method("POST")
                .uri("/json")
                .header("content-type", "application/json")
                .body(Body::from(nfd_payload))
                .unwrap();
            let _ = app.clone().oneshot(nfc).await.unwrap();
            let _ = app.oneshot(nfd).await.unwrap();
            let values = seen.lock().unwrap().clone();
            assert_eq!(values.len(), 2);
            let nfc_value = values.first().unwrap();
            let nfd_value = values.get(1).unwrap();
            assert_ne!(nfc_value, nfd_value);
            assert_eq!(nfc_value.chars().count(), 4);
            assert_eq!(nfd_value.chars().count(), 5);
        }
    }
}

trait GreetingService {
    fn execute(&self, user: &Username) -> Result<String, GreetError>;
}

impl<P: SalutationProvider> GreetingService for GreetUser<P> {
    fn execute(&self, user: &Username) -> Result<String, GreetError> {
        GreetUser::execute(self, user)
    }
}

impl GreetDriver {
    /// Builds the driver from the injected interactor.
    #[must_use]
    pub fn new<P: SalutationProvider + 'static>(interactor: GreetUser<P>) -> Self {
        Self { interactor: Box::new(interactor) }
    }

    /// Validates `raw_name`, runs the greeting use case, and renders the outcome.
    ///
    /// Validation and use-case errors are captured into the returned
    /// [`CommandOutcome`] rather than propagated, so the presenter has a single
    /// value to render.
    #[must_use]
    pub fn handle(&self, raw_name: &str) -> CommandOutcome {
        match Username::new(raw_name) {
            Ok(user) => self.render_greeting(&user),
            Err(error) => CommandOutcome { message: error.to_string(), success: false },
        }
    }

    fn render_greeting(&self, user: &Username) -> CommandOutcome {
        match self.interactor.execute(user) {
            Ok(message) => CommandOutcome { message, success: true },
            Err(error) => CommandOutcome { message: error.to_string(), success: false },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use use_cases::GreetError;

    struct FixedSalutation(&'static str);

    impl SalutationProvider for FixedSalutation {
        fn salutation(&self) -> Result<String, GreetError> {
            Ok(self.0.to_owned())
        }
    }

    struct MissingSalutation;

    impl SalutationProvider for MissingSalutation {
        fn salutation(&self) -> Result<String, GreetError> {
            Err(GreetError::Unavailable)
        }
    }

    #[test]
    fn test_handle_valid_raw_name_returns_success_outcome() {
        let driver = GreetDriver::new(GreetUser::new(FixedSalutation("Hello")));
        let outcome = driver.handle("ada");
        assert!(outcome.success);
        assert_eq!(outcome.message, "Hello, ada!");
    }

    #[test]
    fn test_handle_empty_raw_name_returns_validation_failure() {
        let driver = GreetDriver::new(GreetUser::new(FixedSalutation("Hello")));
        let outcome = driver.handle("   ");

        assert!(!outcome.success);
        assert_eq!(outcome.message, "username must not be empty");
    }

    #[test]
    fn test_handle_unavailable_salutation_returns_failure_outcome() {
        let driver = GreetDriver::new(GreetUser::new(MissingSalutation));
        let outcome = driver.handle("ada");
        assert!(!outcome.success);
        assert_eq!(outcome.message, "salutation is unavailable");
    }
}
