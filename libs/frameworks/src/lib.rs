//! Frameworks layer.
//!
//! Outward adapters that implement the ports declared by the [`use_cases`] layer.
//! This is where I/O, persistence, and external-service concerns live. It may
//! depend on [`entities`] and [`use_cases`] but nothing above it — adapters are
//! wired into the application only at the composition root.
//!
//! The worked example, [`StaticSalutation`], is a trivial adapter implementing
//! [`use_cases::SalutationProvider`] with a fixed word.

use use_cases::{GreetError, SalutationProvider};

/// Adapter supplying a fixed salutation word.
///
/// A real adapter would read from configuration, a database, or a remote
/// service; this placeholder keeps the wiring demonstrable without any I/O.
pub struct StaticSalutation {
    word: String,
}

impl StaticSalutation {
    /// Builds the adapter from the salutation word to serve.
    pub fn new(word: impl Into<String>) -> Self {
        Self { word: word.into() }
    }
}

impl SalutationProvider for StaticSalutation {
    fn salutation(&self) -> Result<String, GreetError> {
        Ok(self.word.clone())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn static_salutation_returns_configured_word() {
        let adapter = StaticSalutation::new("Hi");
        assert_eq!(adapter.salutation().unwrap(), "Hi");
    }
}

/// HTTP server lifecycle adapter.
pub mod http {
    use std::future::{Future, IntoFuture};

    use axum::Router;
    use tokio::net::TcpListener;

    const HTTP_BIND_ADDRESS: &str = "127.0.0.1:3000";

    /// Outcome returned when the HTTP server lifecycle ends.
    pub enum HttpServerOutcome {
        /// The server stopped normally after graceful shutdown.
        Stopped,
        /// The server could not start or stopped because of a framework error.
        Failed(String),
    }

    impl HttpServerOutcome {
        /// Reports whether the server stopped normally.
        #[must_use]
        pub fn is_success(&self) -> bool {
            matches!(self, Self::Stopped)
        }

        /// Returns the presenter-ready lifecycle message.
        #[must_use]
        pub fn message(&self) -> &str {
            match self {
                Self::Stopped => "HTTP server stopped",
                Self::Failed(message) => message,
            }
        }
    }

    /// Axum server adapter for a router assembled by an interface adapter.
    pub struct HttpServer {
        router: Router,
    }

    impl HttpServer {
        /// Builds a server around an already assembled HTTP router.
        #[must_use]
        pub fn new(router: Router) -> Self {
            Self { router }
        }

        /// Binds the configured loopback address and serves until process interrupt (Ctrl-C).
        pub async fn serve(self) -> HttpServerOutcome {
            self.serve_on_until_ctrl_c(HTTP_BIND_ADDRESS).await
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
            map_server_result(axum::serve(listener, self.router).with_graceful_shutdown(shutdown))
                .await
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
        use super::*;

        #[tokio::test]
        async fn test_http_server_lifecycle_maps_graceful_stop() {
            let outcome = map_server_result(std::future::ready(Ok::<(), &str>(()))).await;

            assert!(matches!(outcome, HttpServerOutcome::Stopped));
            assert!(outcome.is_success());
            assert_eq!(outcome.message(), "HTTP server stopped");
        }

        #[tokio::test]
        async fn test_http_server_serve_listener_happy_path_exchanges_then_stops() {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            use tokio::sync::oneshot;
            use tokio::time::{Duration, timeout};

            let exchange = async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();
                let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
                let serve = tokio::spawn(async move {
                    HttpServer::new(Router::new())
                        .serve_listener(listener, async {
                            let _ = shutdown_rx.await;
                        })
                        .await
                });

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
                    assert!(response.len() <= 4096, "HTTP response exceeded byte cap");
                    if response.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                assert!(!response.is_empty(), "expected an HTTP response from axum");
                let response = std::str::from_utf8(&response).unwrap();
                assert!(response.starts_with("HTTP/1.1"), "unexpected response: {response:?}");

                shutdown_tx.send(()).unwrap();
                let outcome = serve.await.unwrap();
                assert!(matches!(outcome, HttpServerOutcome::Stopped));
                assert!(outcome.is_success());
            };

            assert!(
                timeout(Duration::from_secs(5), exchange).await.is_ok(),
                "happy-path exchange timed out",
            );
        }

        #[tokio::test]
        async fn test_http_server_serve_maps_bind_failure() {
            let outcome =
                HttpServer::new(Router::new()).serve_on("127.0.0.1:65536", async {}).await;

            assert!(!outcome.is_success());
            assert!(outcome.message().contains("65536"));
        }

        #[test]
        fn test_http_server_outcome_preserves_failure_diagnostic() {
            let outcome = HttpServerOutcome::Failed("listener closed".to_owned());

            assert_eq!(outcome.message(), "listener closed");
            assert!(!outcome.is_success());
        }
    }
}

/// Authentication framework adapters.
pub mod auth {
    use std::collections::HashMap;
    use std::sync::RwLock;

    use argon2::{Algorithm, Argon2, Block, Params, Version};
    use base64ct::{Base64Unpadded, Encoding};
    use entities::auth::{
        AccessTokenRecord, AccessTokenRepository, AccessTokenRepositoryError, OpaqueAccessToken,
        PasswordHash, User, UserLookupError, UserRepository, UserRepositoryError,
    };
    use rand::{TryRngCore, rngs::OsRng as RandomOsRng};
    use use_cases::auth::{
        OpaqueTokenIssuer, PasswordHasher, PasswordHashingError, PasswordVerificationError,
        PasswordVerifier, PlaintextPassword, TokenIssuanceError,
    };

    /// In-memory user-repository adapter.
    pub struct InMemoryUserRepository {
        users: RwLock<HashMap<String, User>>,
    }

    impl InMemoryUserRepository {
        /// Creates an empty in-memory user repository.
        #[must_use]
        pub fn new() -> Self {
            Self { users: RwLock::new(HashMap::new()) }
        }
    }

    impl Default for InMemoryUserRepository {
        fn default() -> Self {
            Self::new()
        }
    }

    impl UserRepository for InMemoryUserRepository {
        fn find_by_username(
            &self,
            username: &entities::Username,
        ) -> Result<Option<User>, UserLookupError> {
            let users = self.users.read().map_err(|_| UserLookupError::Unavailable)?;
            Ok(users.get(username.as_str()).cloned())
        }

        fn save(&self, user: User) -> Result<(), UserRepositoryError> {
            let mut users = self.users.write().map_err(|_| UserRepositoryError::Unavailable)?;
            if users.contains_key(user.username().as_str()) {
                return Err(UserRepositoryError::DuplicateUser);
            }
            users.insert(user.username().as_str().to_owned(), user);
            Ok(())
        }
    }

    /// In-memory access-token repository adapter.
    pub struct InMemoryAccessTokenRepository {
        tokens: RwLock<HashMap<String, AccessTokenRecord>>,
    }

    impl InMemoryAccessTokenRepository {
        /// Creates an empty in-memory access-token repository.
        #[must_use]
        pub fn new() -> Self {
            Self { tokens: RwLock::new(HashMap::new()) }
        }
    }

    impl Default for InMemoryAccessTokenRepository {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AccessTokenRepository for InMemoryAccessTokenRepository {
        fn save(&self, token: AccessTokenRecord) -> Result<(), AccessTokenRepositoryError> {
            let mut tokens =
                self.tokens.write().map_err(|_| AccessTokenRepositoryError::Unavailable)?;
            tokens.insert(token.token().as_str().to_owned(), token);
            Ok(())
        }

        fn find(
            &self,
            token: &OpaqueAccessToken,
        ) -> Result<Option<AccessTokenRecord>, AccessTokenRepositoryError> {
            let tokens = self.tokens.read().map_err(|_| AccessTokenRepositoryError::Unavailable)?;
            Ok(tokens.get(token.as_str()).cloned())
        }
    }

    /// Argon2id adapter for password hashing and verification.
    #[derive(Default)]
    pub struct Argon2PasswordAdapter;

    const ARGON2_MEMORY_COST: u32 = Params::DEFAULT_M_COST;
    const ARGON2_TIME_COST: u32 = Params::DEFAULT_T_COST;
    const ARGON2_PARALLELISM: u32 = Params::DEFAULT_P_COST;
    const ARGON2_OUTPUT_LENGTH: usize = Params::DEFAULT_OUTPUT_LEN;
    const ARGON2_SALT_LENGTH: usize = 16;
    const ARGON2_MAX_MEMORY_COST: u32 = ARGON2_MEMORY_COST;
    const ARGON2_MAX_TIME_COST: u32 = ARGON2_TIME_COST;
    const ARGON2_MAX_PARALLELISM: u32 = ARGON2_PARALLELISM;

    impl Argon2PasswordAdapter {
        /// Creates an Argon2id password adapter.
        #[must_use]
        pub fn new() -> Self {
            Self
        }
    }

    impl PasswordHasher for Argon2PasswordAdapter {
        fn hash(&self, password: &PlaintextPassword) -> Result<PasswordHash, PasswordHashingError> {
            let mut salt = [0_u8; ARGON2_SALT_LENGTH];
            let mut rng = RandomOsRng;
            rng.try_fill_bytes(&mut salt).map_err(|_| PasswordHashingError::Unavailable)?;

            let params = Params::new(
                ARGON2_MEMORY_COST,
                ARGON2_TIME_COST,
                ARGON2_PARALLELISM,
                Some(ARGON2_OUTPUT_LENGTH),
            )
            .map_err(|_| PasswordHashingError::Unavailable)?;
            let mut digest = vec![0_u8; ARGON2_OUTPUT_LENGTH];
            hash_into(password.expose_secret().as_bytes(), &salt, &params, &mut digest)
                .map_err(|_| PasswordHashingError::Unavailable)?;

            let encoded = format!(
                "$argon2id$v=19$m={ARGON2_MEMORY_COST},t={ARGON2_TIME_COST},p={ARGON2_PARALLELISM}${}${}",
                Base64Unpadded::encode_string(&salt),
                Base64Unpadded::encode_string(&digest)
            );
            PasswordHash::new(encoded).map_err(|_| PasswordHashingError::Unavailable)
        }
    }

    impl PasswordVerifier for Argon2PasswordAdapter {
        fn verify(
            &self,
            password: &PlaintextPassword,
            hash: &PasswordHash,
        ) -> Result<bool, PasswordVerificationError> {
            let parsed =
                parse_encoded_hash(hash.as_str()).ok_or(PasswordVerificationError::Unavailable)?;
            let mut digest = vec![0_u8; parsed.digest.len()];
            hash_into(
                password.expose_secret().as_bytes(),
                &parsed.salt,
                &parsed.params,
                &mut digest,
            )
            .map_err(|_| PasswordVerificationError::Unavailable)?;

            Ok(constant_time_equal(&digest, &parsed.digest))
        }
    }

    struct ParsedPasswordHash {
        params: Params,
        salt: Vec<u8>,
        digest: Vec<u8>,
    }

    fn params_within_limits(params: &Params) -> bool {
        params.m_cost() <= ARGON2_MAX_MEMORY_COST
            && params.t_cost() <= ARGON2_MAX_TIME_COST
            && params.p_cost() <= ARGON2_MAX_PARALLELISM
    }

    fn hash_into(
        password: &[u8],
        salt: &[u8],
        params: &Params,
        digest: &mut [u8],
    ) -> Result<(), argon2::Error> {
        // Application-level bound: never allocate Argon2 memory for out-of-policy params,
        // even if a caller constructed Params outside parse_encoded_hash.
        if !params_within_limits(params) {
            return Err(argon2::Error::MemoryTooLittle);
        }
        let mut memory = vec![Block::default(); params.block_count()];
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params.clone())
            .hash_password_into_with_memory(password, salt, digest, &mut memory)
    }

    /// Fail-closed upper bound on the full PHC encoding length before any parse/scan work.
    const MAX_PHC_ENCODED_BYTES: usize = 256;

    fn parse_encoded_hash(encoded: &str) -> Option<ParsedPasswordHash> {
        if encoded.len() > MAX_PHC_ENCODED_BYTES {
            return None;
        }
        let mut fields = encoded.split('$');
        if fields.next() != Some("") || fields.next() != Some("argon2id") {
            return None;
        }
        if fields.next() != Some("v=19") {
            return None;
        }
        let params_field = fields.next()?;
        let salt_field = fields.next()?;
        let digest_field = fields.next()?;
        if fields.next().is_some() {
            return None;
        }

        let (memory_cost, time_cost, parallelism) = parse_params(params_field)?;
        if memory_cost > ARGON2_MAX_MEMORY_COST
            || time_cost > ARGON2_MAX_TIME_COST
            || parallelism > ARGON2_MAX_PARALLELISM
        {
            return None;
        }
        // Reject oversized encoded salt/digest before Base64 decode / heap allocation.
        // Unpadded Base64 length for N bytes is 4*ceil(N/3) minus padding chars:
        // 16 bytes -> 22 chars; 32 bytes -> 43 chars.
        const MAX_SALT_CHARS: usize = 22;
        const MAX_DIGEST_CHARS: usize = 43;
        if salt_field.len() > MAX_SALT_CHARS || digest_field.len() > MAX_DIGEST_CHARS {
            return None;
        }
        let salt = Base64Unpadded::decode_vec(salt_field).ok()?;
        let digest = Base64Unpadded::decode_vec(digest_field).ok()?;
        if salt.len() > ARGON2_SALT_LENGTH || digest.len() > ARGON2_OUTPUT_LENGTH {
            return None;
        }
        let params = Params::new(memory_cost, time_cost, parallelism, Some(digest.len())).ok()?;

        Some(ParsedPasswordHash { params, salt, digest })
    }

    fn parse_params(params: &str) -> Option<(u32, u32, u32)> {
        let mut memory_cost = None;
        let mut time_cost = None;
        let mut parallelism = None;
        for pair in params.split(',') {
            let (name, value) = pair.split_once('=')?;
            let value = value.parse().ok()?;
            match name {
                "m" if memory_cost.is_none() => memory_cost = Some(value),
                "t" if time_cost.is_none() => time_cost = Some(value),
                "p" if parallelism.is_none() => parallelism = Some(value),
                _ => return None,
            }
        }
        Some((memory_cost?, time_cost?, parallelism?))
    }

    fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
        if left.len() != right.len() {
            return false;
        }
        let mut difference = 0_u8;
        for (left_byte, right_byte) in left.iter().zip(right) {
            difference |= left_byte ^ right_byte;
        }
        difference == 0
    }

    /// OS-random adapter for opaque access-token issuance.
    #[derive(Default)]
    pub struct RandomOpaqueTokenIssuer;

    impl RandomOpaqueTokenIssuer {
        /// Creates a random opaque-token issuer.
        #[must_use]
        pub fn new() -> Self {
            Self
        }
    }

    impl OpaqueTokenIssuer for RandomOpaqueTokenIssuer {
        fn issue(&self) -> Result<OpaqueAccessToken, TokenIssuanceError> {
            let mut bytes = [0_u8; 32];
            let mut rng = RandomOsRng;
            rng.try_fill_bytes(&mut bytes).map_err(|_| TokenIssuanceError::Unavailable)?;

            let mut encoded = String::with_capacity(bytes.len() * 2);
            for byte in bytes {
                encoded.push(hex_digit(byte >> 4));
                encoded.push(hex_digit(byte & 0x0f));
            }
            OpaqueAccessToken::new(encoded).map_err(|_| TokenIssuanceError::Unavailable)
        }
    }

    fn hex_digit(value: u8) -> char {
        match value {
            0..=9 => char::from(b'0' + value),
            10..=15 => char::from(b'a' + value - 10),
            _ => '0',
        }
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use super::*;
        use entities::Username;

        fn valid_hash() -> PasswordHash {
            PasswordHash::new("$argon2id$v=19$m=19456,t=2,p=1$salt$hash".to_owned()).unwrap()
        }

        fn valid_user() -> User {
            User::new(Username::new("ada").unwrap(), valid_hash())
        }

        #[test]
        fn test_in_memory_user_repository_save_then_find_returns_user() {
            let repository = InMemoryUserRepository::new();
            let user = valid_user();

            repository.save(user.clone()).unwrap();

            let found = repository.find_by_username(user.username()).unwrap();
            assert_eq!(found.as_ref().map(User::username), Some(user.username()));
        }

        #[test]
        fn test_in_memory_user_repository_missing_username_returns_none() {
            let repository = InMemoryUserRepository::new();
            let username = Username::new("ada").unwrap();

            assert!(repository.find_by_username(&username).unwrap().is_none());
        }

        #[test]
        fn test_in_memory_user_repository_duplicate_username_returns_duplicate_error() {
            let repository = InMemoryUserRepository::new();
            let user = valid_user();
            repository.save(user.clone()).unwrap();

            assert_eq!(repository.save(user), Err(UserRepositoryError::DuplicateUser));
        }

        #[test]
        fn test_in_memory_access_token_repository_save_then_find_returns_record() {
            let repository = InMemoryAccessTokenRepository::new();
            let record = AccessTokenRecord::new(
                OpaqueAccessToken::new("opaque".to_owned()).unwrap(),
                Username::new("ada").unwrap(),
            );
            let token = record.token().clone();

            repository.save(record.clone()).unwrap();

            let found = repository.find(&token).unwrap();
            assert_eq!(
                found.as_ref().map(|stored| stored.subject().as_str()),
                Some(record.subject().as_str())
            );
        }

        #[test]
        fn test_in_memory_access_token_repository_missing_token_returns_none() {
            let repository = InMemoryAccessTokenRepository::new();
            let token = OpaqueAccessToken::new("missing".to_owned()).unwrap();

            assert!(repository.find(&token).unwrap().is_none());
        }

        #[test]
        fn test_argon2_password_adapter_hash_returns_argon2id_hash() {
            let adapter = Argon2PasswordAdapter::new();
            let password = PlaintextPassword::parse("correct horse").unwrap();

            let hash = adapter.hash(&password).unwrap();

            assert!(hash.is_argon2id());
            assert_ne!(hash.as_str(), password.expose_secret());
        }

        #[test]
        fn test_argon2_password_adapter_verify_matching_password_returns_true() {
            let adapter = Argon2PasswordAdapter::new();
            let password = PlaintextPassword::parse("correct horse").unwrap();
            let hash = adapter.hash(&password).unwrap();

            assert!(adapter.verify(&password, &hash).unwrap());
        }

        #[test]
        fn test_argon2_password_adapter_verify_wrong_password_returns_false() {
            let adapter = Argon2PasswordAdapter::new();
            let password = PlaintextPassword::parse("correct horse").unwrap();
            let wrong_password = PlaintextPassword::parse("wrong horse").unwrap();
            let hash = adapter.hash(&password).unwrap();

            assert!(!adapter.verify(&wrong_password, &hash).unwrap());
        }

        #[test]
        fn test_argon2_password_adapter_verify_rejects_unbounded_parameters() {
            let adapter = Argon2PasswordAdapter::new();
            let password = PlaintextPassword::parse("correct horse").unwrap();
            let hash =
                PasswordHash::new("$argon2id$v=19$m=4294967295,t=2,p=1$c2FsdA$aGFzaA".to_owned())
                    .unwrap();

            assert_eq!(
                adapter.verify(&password, &hash),
                Err(PasswordVerificationError::Unavailable)
            );
        }

        #[test]
        fn test_argon2_password_adapter_verify_rejects_oversized_digest_field() {
            // Shared with entities PasswordHash construction (43-char digest cap).
            let oversized = "A".repeat(128);
            let encoded = format!("$argon2id$v=19$m=19456,t=2,p=1$c2FsdA${oversized}");
            assert!(PasswordHash::new(encoded).is_err());
        }

        #[test]
        fn test_argon2_password_adapter_verify_rejects_oversized_phc_encoding() {
            // Shared with entities PasswordHash construction (256-byte PHC cap).
            let long_salt = "A".repeat(220);
            let encoded = format!(
                "$argon2id$v=19$m=19456,t=2,p=1${long_salt}$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            );
            assert!(encoded.len() > 256);
            assert!(PasswordHash::new(encoded).is_err());
        }

        #[test]
        fn test_argon2_password_adapter_verify_rejects_unpadded_length_boundary_overflows() {
            use base64ct::{Base64Unpadded, Encoding};

            let adapter = Argon2PasswordAdapter::new();
            let password = PlaintextPassword::parse("correct horse").unwrap();
            // 17-byte salt is accepted by PasswordHash construction but rejected by frameworks.
            let long_salt = Base64Unpadded::encode_string(&[0_u8; 17]);
            let digest = Base64Unpadded::encode_string(&[0_u8; 32]);
            let hash =
                PasswordHash::new(format!("$argon2id$v=19$m=19456,t=2,p=1${long_salt}${digest}"))
                    .unwrap();
            assert_eq!(
                adapter.verify(&password, &hash),
                Err(PasswordVerificationError::Unavailable)
            );

            // 44-char digest exceeds the shared 43-char construction guard.
            let long_digest = "A".repeat(44);
            assert!(
                PasswordHash::new(format!(
                    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdGhlc2FsdHZhbA${long_digest}"
                ))
                .is_err()
            );
        }

        #[test]
        fn test_random_opaque_token_issuer_issue_returns_non_empty_unique_tokens() {
            let issuer = RandomOpaqueTokenIssuer::new();

            let first = issuer.issue().unwrap();
            let second = issuer.issue().unwrap();

            assert!(first.is_non_empty());
            assert!(second.is_non_empty());
            assert_ne!(first.as_str(), second.as_str());
        }
    }
}
