//! CLI composition root.
//!
//! The only place allowed to know every layer at once. It constructs the
//! concrete infrastructure adapter, injects it into the usecase interactor,
//! and wires that into the CLI driver.

use cli_driver::GreetDriver;
use infrastructure::StaticSalutation;
use usecase::GreetUser;

/// Composition root for the greeting command.
pub struct GreetingCompositionRoot;

impl GreetingCompositionRoot {
    /// Creates a greeting composition root.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Builds the fully wired greeting primary adapter.
    #[must_use]
    pub fn greeting_driver(&self) -> GreetDriver {
        let adapter = StaticSalutation::new("Hello");
        let interactor = GreetUser::new(adapter);

        GreetDriver::new(interactor)
    }
}

impl Default for GreetingCompositionRoot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting_driver_wires_all_layers() {
        let root = GreetingCompositionRoot::new();
        let outcome = root.greeting_driver().handle("ada");
        assert!(outcome.success);
        assert_eq!(outcome.message, "Hello, ada!");
    }

    #[test]
    fn test_greeting_driver_rejects_empty_name() {
        let root = GreetingCompositionRoot::new();
        let outcome = root.greeting_driver().handle("   ");

        assert!(!outcome.success);
        assert_eq!(outcome.message, "username must not be empty");
    }
}

/// Authentication composition root.
pub mod auth {
    use std::sync::Arc;

    use cli_driver::auth::AuthHttpApi;
    use infrastructure::auth::{
        Argon2PasswordAdapter, InMemoryAccessTokenRepository, InMemoryUserRepository,
        RandomOpaqueTokenIssuer,
    };
    use usecase::auth::{
        LoginInteractor, LoginService, PasswordHasher, PasswordVerifier, RegisterUserInteractor,
        RegisterUserService,
    };

    /// Pure DI composition root for the minimal auth HTTP API.
    pub struct AuthCompositionRoot;

    impl AuthCompositionRoot {
        /// Creates an auth composition root.
        #[must_use]
        pub fn new() -> Self {
            Self
        }

        /// Builds the fully wired auth HTTP primary adapter.
        #[must_use]
        pub fn auth_http_api(&self) -> AuthHttpApi {
            let users = Arc::new(InMemoryUserRepository::new());
            let tokens = Arc::new(InMemoryAccessTokenRepository::new());
            let argon2 = Arc::new(Argon2PasswordAdapter::new());
            let hasher: Arc<dyn PasswordHasher> = Arc::clone(&argon2) as Arc<dyn PasswordHasher>;
            let verifier: Arc<dyn PasswordVerifier> =
                Arc::clone(&argon2) as Arc<dyn PasswordVerifier>;
            let issuer = Arc::new(RandomOpaqueTokenIssuer::new());

            let register: Arc<dyn RegisterUserService> =
                Arc::new(RegisterUserInteractor::new(Arc::clone(&users) as _, hasher));
            let login: Arc<dyn LoginService> =
                Arc::new(LoginInteractor::new(users as _, verifier, issuer as _, tokens as _));

            AuthHttpApi::new(register, login)
        }
    }

    impl Default for AuthCompositionRoot {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;

        use super::*;

        #[tokio::test]
        async fn test_auth_composition_root_register_then_login_issues_token() {
            let app = AuthCompositionRoot::new().auth_http_api().router();

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
            let response = app.oneshot(login).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let token = body.get("access_token").and_then(|v| v.as_str()).unwrap();
            assert!(!token.is_empty());
        }

        #[tokio::test]
        async fn test_auth_composition_root_login_rejects_unknown_user() {
            let app = AuthCompositionRoot::new().auth_http_api().router();
            let login = Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"missing","password":"correct horse"}"#))
                .unwrap();
            assert_eq!(app.oneshot(login).await.unwrap().status(), StatusCode::UNAUTHORIZED);
        }
    }
}
