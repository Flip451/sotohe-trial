//! CLI driver layer (primary adapter).
//!
//! Holds injected use cases, invokes them, and renders their result into a
//! transport-neutral [`CommandOutcome`]. It depends only on [`usecase`]; domain
//! types it needs are reached through `usecase`'s re-exports, so it never takes
//! a direct dependency on the domain crate.

use usecase::{GreetError, GreetUser, SalutationProvider, Username};

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
    use std::future::{Future, IntoFuture};
    use std::sync::Arc;

    use axum::{
        Router,
        body::Bytes,
        extract::{DefaultBodyLimit, State},
        http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
        routing::post,
    };
    use tokio::net::TcpListener;
    use usecase::auth::{
        CredentialParseError, LoginService, RegisterUserCommand, RegisterUserError,
        RegisterUserService,
    };

    use super::CommandOutcome;

    const HTTP_BIND_ADDRESS: &str = "127.0.0.1:3000";
    const HTTP_MAX_BODY_BYTES: usize = 64 * 1024;

    /// Outcome returned when the HTTP server lifecycle ends.
    pub enum HttpServerOutcome {
        /// The server stopped normally after graceful shutdown.
        Stopped,
        /// The server could not start or stopped because of an infrastructure error.
        Failed(String),
    }

    impl HttpServerOutcome {
        fn into_command_outcome(self) -> CommandOutcome {
            match self {
                Self::Stopped => {
                    CommandOutcome { message: "HTTP server stopped".to_owned(), success: true }
                }
                Self::Failed(message) => CommandOutcome { message, success: false },
            }
        }
    }

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

    impl AuthHttpApi {
        /// Builds the HTTP adapter from the registration and login services.
        #[must_use]
        pub fn new(register: Arc<dyn RegisterUserService>, login: Arc<dyn LoginService>) -> Self {
            Self { register, login }
        }

        /// Binds the REST server and serves until process interrupt (Ctrl-C).
        ///
        /// Renders the lifecycle into [`CommandOutcome`] for the process presenter.
        pub async fn serve(self) -> CommandOutcome {
            self.serve_on_until_ctrl_c(HTTP_BIND_ADDRESS).await.into_command_outcome()
        }

        async fn serve_on_until_ctrl_c(self, address: &str) -> HttpServerOutcome {
            let listener = match TcpListener::bind(address).await {
                Ok(listener) => listener,
                Err(error) => {
                    return HttpServerOutcome::Failed(format!(
                        "failed to bind HTTP server on {address}: {error}"
                    ));
                }
            };

            let signal_error = std::sync::Arc::new(std::sync::Mutex::new(None));
            let signal_error_for_shutdown = std::sync::Arc::clone(&signal_error);
            let shutdown = async move {
                if let Err(error) = tokio::signal::ctrl_c().await {
                    *signal_error_for_shutdown
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error);
                }
            };

            let served = self.serve_listener(listener, shutdown).await;
            if let Some(error) =
                signal_error.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take()
            {
                return HttpServerOutcome::Failed(format!(
                    "failed to install Ctrl-C handler: {error}"
                ));
            }
            served
        }

        #[cfg(test)]
        async fn serve_on(
            self,
            address: &str,
            shutdown: impl Future<Output = ()> + Send + 'static,
        ) -> HttpServerOutcome {
            let listener = match TcpListener::bind(address).await {
                Ok(listener) => listener,
                Err(error) => {
                    return HttpServerOutcome::Failed(format!(
                        "failed to bind HTTP server on {address}: {error}"
                    ));
                }
            };

            self.serve_listener(listener, shutdown).await
        }

        async fn serve_listener(
            self,
            listener: TcpListener,
            shutdown: impl Future<Output = ()> + Send + 'static,
        ) -> HttpServerOutcome {
            map_server_result(axum::serve(listener, self.router()).with_graceful_shutdown(shutdown))
                .await
        }

        fn router(&self) -> Router {
            Router::new()
                .route("/register", post(register_user))
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

        match register.execute(request.into_command()) {
            Ok(()) => StatusCode::CREATED,
            Err(RegisterUserError::UserAlreadyExists) => StatusCode::CONFLICT,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    async fn map_server_result<S, E>(server: S) -> HttpServerOutcome
    where
        S: IntoFuture<Output = Result<(), E>>,
        E: ToString,
    {
        match server.into_future().await {
            Ok(()) => HttpServerOutcome::Stopped,
            Err(error) => HttpServerOutcome::Failed(error.to_string()),
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
        use tokio::sync::oneshot;
        use tower::ServiceExt;
        use usecase::auth::{
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
                .body(Body::from(r#"{"username":"café","password":"secret"}"#))
                .unwrap();
            let nfd_payload = "{\"username\":\"cafe\\u0301\",\"password\":\"secret\"}";
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
            assert_eq!(nfc.1, "secret");
            assert_eq!(nfd.1, "secret");
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

            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
            let serve = tokio::spawn(async move {
                api()
                    .serve_listener(listener, async {
                        let _ = shutdown_rx.await;
                    })
                    .await
            });

            // Readiness: an HTTP exchange proves Axum accepted and responded (404 on empty router).
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            stream
                .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            let mut response = Vec::new();
            let mut buf = [0_u8; 64];
            loop {
                let n = stream.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                if let Some(chunk) = buf.get(..n) {
                    response.extend_from_slice(chunk);
                }
                if response.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            assert!(!response.is_empty(), "expected an HTTP response from axum");
            let response = std::str::from_utf8(&response).unwrap();
            assert!(response.starts_with("HTTP/1.1"), "unexpected response: {response:?}");
            shutdown_tx.send(()).unwrap();
            let outcome = serve.await.unwrap();
            assert!(matches!(outcome, HttpServerOutcome::Stopped));
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

        #[tokio::test]
        async fn test_auth_http_api_server_lifecycle_maps_graceful_stop() {
            let outcome = map_server_result(std::future::ready(Ok::<(), &str>(()))).await;

            assert!(matches!(outcome, HttpServerOutcome::Stopped));
            let rendered = outcome.into_command_outcome();
            assert!(rendered.success);
        }

        #[tokio::test]
        async fn test_auth_http_api_serve_maps_bind_failure() {
            let outcome = api().serve_on("127.0.0.1:65536", async {}).await.into_command_outcome();

            assert!(!outcome.success);
            assert!(outcome.message.contains("65536"));
        }

        #[test]
        fn test_http_server_outcome_preserves_failure_diagnostic() {
            let failed_message = match HttpServerOutcome::Failed("listener closed".to_owned()) {
                HttpServerOutcome::Failed(message) => message,
                HttpServerOutcome::Stopped => String::new(),
            };
            assert_eq!(failed_message, "listener closed");
            assert!(matches!(HttpServerOutcome::Stopped, HttpServerOutcome::Stopped));
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
    use usecase::GreetError;

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
