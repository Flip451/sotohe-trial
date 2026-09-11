//! Usecase layer.
//!
//! Application-specific business rules. This layer orchestrates domain types
//! and depends **only** on the [`domain`] crate. Outward dependencies (I/O,
//! persistence, external services) are expressed as *ports* — traits defined
//! here and implemented by adapters in the infrastructure layer.
//!
//! The worked example pairs one port ([`SalutationProvider`]) with one
//! interactor ([`GreetUser`]) that composes the port's output with a
//! [`domain::Username`].

use thiserror::Error;

// Re-export the domain types that appear in this layer's public API so that
// downstream layers depending only on `usecase` (e.g. the CLI driver) can name
// them without taking a direct dependency on `domain`.
pub use domain::{Username, UsernameError};

/// Error returned when a greeting cannot be produced.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GreetError {
    /// The salutation source was unavailable.
    #[error("salutation is unavailable")]
    Unavailable,
}

/// Port supplying the salutation word used to greet a user (e.g. `"Hello"`).
///
/// The usecase layer owns this contract; an infrastructure adapter provides the
/// concrete implementation, keeping I/O concerns out of the business rules.
pub trait SalutationProvider {
    /// Returns the salutation word to prefix a greeting with.
    ///
    /// # Errors
    ///
    /// Returns [`GreetError::Unavailable`] when no salutation can be supplied.
    fn salutation(&self) -> Result<String, GreetError>;
}

/// Interactor that greets a [`Username`] using an injected [`SalutationProvider`].
pub struct GreetUser<P: SalutationProvider> {
    provider: P,
}

impl<P: SalutationProvider> GreetUser<P> {
    /// Builds the interactor from its injected port dependency.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Produces a greeting for `user`.
    ///
    /// # Errors
    ///
    /// Propagates [`GreetError`] when the [`SalutationProvider`] cannot supply a
    /// salutation.
    pub fn execute(&self, user: &Username) -> Result<String, GreetError> {
        let word = self.provider.salutation()?;
        Ok(format!("{word}, {}!", user.as_str()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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
    fn test_execute_successful_salutation_returns_greeting() {
        let interactor = GreetUser::new(FixedSalutation("Hello"));
        let user = Username::new("ada").unwrap();
        assert_eq!(interactor.execute(&user).unwrap(), "Hello, ada!");
    }

    #[test]
    fn test_execute_missing_salutation_returns_unavailable_error() {
        let interactor = GreetUser::new(MissingSalutation);
        let user = Username::new("ada").unwrap();
        assert_eq!(interactor.execute(&user), Err(GreetError::Unavailable));
    }
}

/// Authentication application commands, ports, and interactors.
pub mod auth {
    use std::sync::Arc;

    use thiserror::Error;

    use domain::auth::{
        AccessTokenRecord, AccessTokenRepository, AccessTokenRepositoryError, OpaqueAccessToken,
        PasswordHash, User, UserLookupError, UserRepository, UserRepositoryError,
    };
    use domain::{Username, UsernameError};

    /// Maximum accepted UTF-8 byte length for an application-boundary password.
    pub const MAX_PLAINTEXT_PASSWORD_BYTES: usize = 1024;

    /// Failure returned when an application-boundary password is empty or too long.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum PasswordInputError {
        /// The password input was empty.
        #[error("password must not be empty")]
        Empty,
        /// The password exceeded [`MAX_PLAINTEXT_PASSWORD_BYTES`].
        #[error("password must be at most {MAX_PLAINTEXT_PASSWORD_BYTES} UTF-8 bytes")]
        TooLong,
    }

    /// A transient, non-empty password accepted at the application boundary.
    #[derive(PartialEq, Eq)]
    pub struct PlaintextPassword {
        value: String,
    }

    impl PlaintextPassword {
        /// Parses a raw password without changing its contents.
        ///
        /// # Errors
        ///
        /// Returns [`PasswordInputError::Empty`] when `raw` is empty, or
        /// [`PasswordInputError::TooLong`] when `raw` exceeds
        /// [`MAX_PLAINTEXT_PASSWORD_BYTES`] UTF-8 bytes.
        pub fn parse(raw: &str) -> Result<Self, PasswordInputError> {
            if raw.is_empty() {
                return Err(PasswordInputError::Empty);
            }
            if raw.len() > MAX_PLAINTEXT_PASSWORD_BYTES {
                return Err(PasswordInputError::TooLong);
            }
            Ok(Self { value: raw.to_owned() })
        }

        /// Exposes the password only to a hashing or verification port.
        #[must_use]
        pub fn expose_secret(&self) -> &str {
            &self.value
        }

        /// Reports whether the password is non-empty.
        #[must_use]
        pub fn is_non_empty(&self) -> bool {
            !self.value.is_empty()
        }
    }

    /// Failure produced while parsing registration or login credentials.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum CredentialParseError {
        /// The username could not be parsed as a domain username.
        #[error("invalid username: {0}")]
        InvalidUsername(#[from] UsernameError),
        /// The password could not be parsed at the application boundary.
        #[error("invalid password: {0}")]
        InvalidPassword(#[from] PasswordInputError),
    }

    /// Validated registration input owned by the application boundary.
    #[derive(PartialEq, Eq)]
    pub struct RegisterUserCommand {
        username: Username,
        password: PlaintextPassword,
    }

    impl RegisterUserCommand {
        /// Parses a registration request into typed credentials.
        ///
        /// # Errors
        ///
        /// Returns [`CredentialParseError`] when either credential is invalid.
        pub fn parse(username: &str, password: &str) -> Result<Self, CredentialParseError> {
            let username = Username::new(username).map_err(CredentialParseError::from)?;
            let password =
                PlaintextPassword::parse(password).map_err(CredentialParseError::from)?;
            Ok(Self { username, password })
        }

        /// Returns the validated username.
        #[must_use]
        pub fn username(&self) -> &Username {
            &self.username
        }

        /// Returns the transient password for port consumption.
        #[must_use]
        pub fn password(&self) -> &PlaintextPassword {
            &self.password
        }
    }

    /// Validated login input owned by the application boundary.
    #[derive(PartialEq, Eq)]
    pub struct LoginCommand {
        username: Username,
        password: PlaintextPassword,
    }

    impl LoginCommand {
        /// Parses a login request into typed credentials.
        ///
        /// # Errors
        ///
        /// Returns [`CredentialParseError`] when either credential is invalid.
        pub fn parse(username: &str, password: &str) -> Result<Self, CredentialParseError> {
            let username = Username::new(username).map_err(CredentialParseError::from)?;
            let password =
                PlaintextPassword::parse(password).map_err(CredentialParseError::from)?;
            Ok(Self { username, password })
        }

        /// Returns the validated username.
        #[must_use]
        pub fn username(&self) -> &Username {
            &self.username
        }

        /// Returns the transient password for port consumption.
        #[must_use]
        pub fn password(&self) -> &PlaintextPassword {
            &self.password
        }
    }

    /// Failure returned by the password-hashing port.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum PasswordHashingError {
        /// Hashing was unavailable.
        #[error("password hashing is unavailable")]
        Unavailable,
    }

    /// Failure returned by the password-verification port.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum PasswordVerificationError {
        /// Verification was unavailable.
        #[error("password verification is unavailable")]
        Unavailable,
    }

    /// Failure returned by the opaque-token issuance port.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum TokenIssuanceError {
        /// Token issuance was unavailable.
        #[error("token issuance is unavailable")]
        Unavailable,
    }

    /// Port that converts a transient plaintext password into a stored hash.
    pub trait PasswordHasher: Send + Sync {
        /// Hashes a validated password.
        ///
        /// # Errors
        ///
        /// Returns [`PasswordHashingError::Unavailable`] when hashing cannot
        /// be completed.
        fn hash(&self, password: &PlaintextPassword) -> Result<PasswordHash, PasswordHashingError>;
    }

    /// Port that checks a transient password against a stored hash.
    pub trait PasswordVerifier: Send + Sync {
        /// Verifies a password against a stored hash.
        ///
        /// # Errors
        ///
        /// Returns [`PasswordVerificationError::Unavailable`] when the
        /// verification operation cannot be completed.
        fn verify(
            &self,
            password: &PlaintextPassword,
            hash: &PasswordHash,
        ) -> Result<bool, PasswordVerificationError>;
    }

    /// Port that issues unpredictable opaque access tokens.
    pub trait OpaqueTokenIssuer: Send + Sync {
        /// Issues a new opaque access token.
        ///
        /// # Errors
        ///
        /// Returns [`TokenIssuanceError::Unavailable`] when issuance cannot be
        /// completed.
        fn issue(&self) -> Result<OpaqueAccessToken, TokenIssuanceError>;
    }

    /// Failure returned by the user-registration interactor.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum RegisterUserError {
        /// A user with the requested username is already stored.
        #[error("user already exists")]
        UserAlreadyExists,
        /// User lookup failed before registration could proceed.
        #[error("user lookup failed: {0}")]
        Lookup(#[from] UserLookupError),
        /// Password hashing failed before the user could be persisted.
        #[error("password hashing failed: {0}")]
        Hashing(#[from] PasswordHashingError),
        /// User persistence failed.
        #[error("user persistence failed: {0}")]
        Persistence(UserRepositoryError),
    }

    impl From<UserRepositoryError> for RegisterUserError {
        fn from(error: UserRepositoryError) -> Self {
            match error {
                UserRepositoryError::DuplicateUser => Self::UserAlreadyExists,
                unavailable @ UserRepositoryError::Unavailable => Self::Persistence(unavailable),
            }
        }
    }

    /// Structure-required inbound port for user registration.
    pub trait RegisterUserService: Send + Sync {
        /// Registers a validated user command.
        ///
        /// # Errors
        ///
        /// Returns [`RegisterUserError`] when lookup, hashing, or persistence
        /// fails, or when the username is already registered.
        fn execute(&self, command: RegisterUserCommand) -> Result<(), RegisterUserError>;
    }

    /// Registration interactor coordinating user lookup, password hashing, and persistence.
    pub struct RegisterUserInteractor {
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
    }

    impl RegisterUserInteractor {
        /// Builds a registration interactor from its collaborators.
        #[must_use]
        pub fn new(users: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
            Self { users, hasher }
        }
    }

    impl RegisterUserService for RegisterUserInteractor {
        fn execute(&self, command: RegisterUserCommand) -> Result<(), RegisterUserError> {
            if self.users.find_by_username(command.username())?.is_some() {
                return Err(RegisterUserError::UserAlreadyExists);
            }

            let password_hash = self.hasher.hash(command.password())?;
            let user = User::new(command.username().clone(), password_hash);
            self.users.save(user)?;

            Ok(())
        }
    }

    /// Application response containing an issued opaque access token.
    #[derive(Clone, PartialEq, Eq)]
    pub struct IssuedAccessToken {
        value: String,
    }

    impl IssuedAccessToken {
        /// Converts a domain token into the application response form.
        #[must_use]
        pub fn from_domain(token: OpaqueAccessToken) -> Self {
            Self { value: token.as_str().to_owned() }
        }

        /// Returns the token representation for presentation.
        #[must_use]
        pub fn as_str(&self) -> &str {
            &self.value
        }
    }

    /// Failure returned by the login interactor.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum LoginError {
        /// No user matched the requested username.
        #[error("user not found")]
        UserNotFound,
        /// The supplied password did not match the stored hash.
        #[error("invalid credentials")]
        InvalidCredentials,
        /// User lookup failed.
        #[error("user lookup failed: {0}")]
        Lookup(#[from] UserLookupError),
        /// Password verification failed.
        #[error("password verification failed: {0}")]
        Verification(#[from] PasswordVerificationError),
        /// Token issuance failed.
        #[error("token issuance failed: {0}")]
        TokenIssuance(#[from] TokenIssuanceError),
        /// Token persistence failed.
        #[error("token persistence failed: {0}")]
        Persistence(#[from] AccessTokenRepositoryError),
    }

    /// Structure-required inbound port for login.
    pub trait LoginService: Send + Sync {
        /// Authenticates a login command and issues an access token.
        ///
        /// # Errors
        ///
        /// Returns [`LoginError`] when lookup, verification, issuance, or
        /// token persistence fails.
        fn execute(&self, command: LoginCommand) -> Result<IssuedAccessToken, LoginError>;
    }

    /// Login interactor coordinating repositories and authentication ports.
    pub struct LoginInteractor {
        users: Arc<dyn UserRepository>,
        verifier: Arc<dyn PasswordVerifier>,
        token_issuer: Arc<dyn OpaqueTokenIssuer>,
        tokens: Arc<dyn AccessTokenRepository>,
    }

    impl LoginInteractor {
        /// Builds a login interactor from its collaborators.
        #[must_use]
        pub fn new(
            users: Arc<dyn UserRepository>,
            verifier: Arc<dyn PasswordVerifier>,
            token_issuer: Arc<dyn OpaqueTokenIssuer>,
            tokens: Arc<dyn AccessTokenRepository>,
        ) -> Self {
            Self { users, verifier, token_issuer, tokens }
        }
    }

    impl LoginService for LoginInteractor {
        fn execute(&self, command: LoginCommand) -> Result<IssuedAccessToken, LoginError> {
            let user =
                self.users.find_by_username(command.username())?.ok_or(LoginError::UserNotFound)?;
            let verified = self.verifier.verify(command.password(), user.password_hash())?;
            if !verified {
                return Err(LoginError::InvalidCredentials);
            }

            let token = self.token_issuer.issue()?;
            let record = AccessTokenRecord::new(token.clone(), user.username().clone());
            self.tokens.save(record)?;

            Ok(IssuedAccessToken::from_domain(token))
        }
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use std::sync::{Arc, Mutex};

        use super::*;
        use domain::auth::User;

        fn valid_hash() -> PasswordHash {
            PasswordHash::new("$argon2id$v=19$m=19456,t=2,p=1$salt$hash".to_owned()).unwrap()
        }

        fn valid_command() -> LoginCommand {
            LoginCommand::parse("ada", "correct horse").unwrap()
        }

        fn valid_user() -> User {
            User::new(Username::new("ada").unwrap(), valid_hash())
        }

        struct RegistrationUserRepository {
            existing: Option<User>,
            lookup_unavailable: bool,
            save_unavailable: bool,
            duplicate_on_save: bool,
            saved: Mutex<Option<User>>,
        }

        impl RegistrationUserRepository {
            fn empty() -> Self {
                Self {
                    existing: None,
                    lookup_unavailable: false,
                    save_unavailable: false,
                    duplicate_on_save: false,
                    saved: Mutex::new(None),
                }
            }
        }

        impl UserRepository for RegistrationUserRepository {
            fn find_by_username(
                &self,
                _username: &Username,
            ) -> Result<Option<User>, UserLookupError> {
                if self.lookup_unavailable {
                    Err(UserLookupError::Unavailable)
                } else {
                    Ok(self.existing.clone())
                }
            }

            fn save(&self, user: User) -> Result<(), domain::auth::UserRepositoryError> {
                if self.duplicate_on_save {
                    return Err(domain::auth::UserRepositoryError::DuplicateUser);
                }
                if self.save_unavailable {
                    return Err(domain::auth::UserRepositoryError::Unavailable);
                }
                *self.saved.lock().unwrap() = Some(user);
                Ok(())
            }
        }

        struct RegistrationPasswordHasher {
            unavailable: bool,
        }

        impl PasswordHasher for RegistrationPasswordHasher {
            fn hash(
                &self,
                _password: &PlaintextPassword,
            ) -> Result<PasswordHash, PasswordHashingError> {
                if self.unavailable {
                    Err(PasswordHashingError::Unavailable)
                } else {
                    Ok(valid_hash())
                }
            }
        }

        fn registration_interactor(
            users: Arc<RegistrationUserRepository>,
            hasher_unavailable: bool,
        ) -> RegisterUserInteractor {
            RegisterUserInteractor::new(
                users,
                Arc::new(RegistrationPasswordHasher { unavailable: hasher_unavailable }),
            )
        }

        fn valid_registration_command() -> RegisterUserCommand {
            RegisterUserCommand::parse("ada", "correct horse").unwrap()
        }

        #[test]
        fn test_register_user_interactor_valid_credentials_hashes_and_persists_user() {
            let users = Arc::new(RegistrationUserRepository::empty());
            let interactor = registration_interactor(users.clone(), false);
            let service: &dyn RegisterUserService = &interactor;

            service.execute(valid_registration_command()).unwrap();

            let saved = users.saved.lock().unwrap();
            let user = saved.as_ref().unwrap();
            assert_eq!(user.username().as_str(), "ada");
            assert_ne!(user.password_hash().as_str(), "correct horse");
            assert!(user.password_hash().is_argon2id());
        }

        #[test]
        fn test_register_user_interactor_existing_user_returns_duplicate_error() {
            let users = Arc::new(RegistrationUserRepository {
                existing: Some(valid_user()),
                ..RegistrationUserRepository::empty()
            });
            let interactor = registration_interactor(users, false);

            assert_eq!(
                interactor.execute(valid_registration_command()),
                Err(RegisterUserError::UserAlreadyExists)
            );
        }

        #[test]
        fn test_register_user_interactor_lookup_failure_returns_lookup_error() {
            let users = Arc::new(RegistrationUserRepository {
                lookup_unavailable: true,
                ..RegistrationUserRepository::empty()
            });
            let interactor = registration_interactor(users, false);

            assert_eq!(
                interactor.execute(valid_registration_command()),
                Err(RegisterUserError::Lookup(UserLookupError::Unavailable))
            );
        }

        #[test]
        fn test_register_user_interactor_hashing_failure_returns_hashing_error() {
            let users = Arc::new(RegistrationUserRepository::empty());
            let interactor = registration_interactor(users.clone(), true);

            assert_eq!(
                interactor.execute(valid_registration_command()),
                Err(RegisterUserError::Hashing(PasswordHashingError::Unavailable))
            );
            assert!(users.saved.lock().unwrap().is_none());
        }

        #[test]
        fn test_register_user_interactor_persistence_failure_returns_persistence_error() {
            let users = Arc::new(RegistrationUserRepository {
                save_unavailable: true,
                ..RegistrationUserRepository::empty()
            });
            let interactor = registration_interactor(users, false);

            assert_eq!(
                interactor.execute(valid_registration_command()),
                Err(RegisterUserError::Persistence(domain::auth::UserRepositoryError::Unavailable))
            );
        }

        #[test]
        fn test_register_user_interactor_save_race_returns_duplicate_error() {
            let users = Arc::new(RegistrationUserRepository {
                duplicate_on_save: true,
                ..RegistrationUserRepository::empty()
            });
            let interactor = registration_interactor(users, false);

            assert_eq!(
                interactor.execute(valid_registration_command()),
                Err(RegisterUserError::UserAlreadyExists)
            );
        }

        #[test]
        fn test_plaintext_password_empty_input_returns_empty_error() {
            assert!(matches!(PlaintextPassword::parse(""), Err(PasswordInputError::Empty)));

            let password = PlaintextPassword::parse(" secret ").unwrap();
            assert!(password.is_non_empty());
            assert_eq!(password.expose_secret(), " secret ");
        }

        #[test]
        fn test_plaintext_password_too_long_input_returns_too_long_error() {
            let too_long = "a".repeat(MAX_PLAINTEXT_PASSWORD_BYTES + 1);
            assert!(matches!(
                PlaintextPassword::parse(&too_long),
                Err(PasswordInputError::TooLong)
            ));

            let at_limit = "a".repeat(MAX_PLAINTEXT_PASSWORD_BYTES);
            assert!(PlaintextPassword::parse(&at_limit).is_ok());
        }

        #[test]
        fn test_register_user_command_empty_password_returns_invalid_password_error() {
            assert!(matches!(
                RegisterUserCommand::parse("ada", ""),
                Err(CredentialParseError::InvalidPassword(PasswordInputError::Empty))
            ));
        }

        #[test]
        fn test_register_user_command_invalid_username_returns_invalid_username_error() {
            assert!(matches!(
                RegisterUserCommand::parse("   ", "secret"),
                Err(CredentialParseError::InvalidUsername(_))
            ));
        }

        #[test]
        fn test_register_user_command_valid_credentials_returns_typed_command() {
            let command = RegisterUserCommand::parse(" ada ", " secret ").unwrap();

            assert_eq!(command.username().as_str(), "ada");
            assert_eq!(command.password().expose_secret(), " secret ");
        }

        #[test]
        fn test_login_command_empty_password_returns_invalid_password_error() {
            assert!(matches!(
                LoginCommand::parse("ada", ""),
                Err(CredentialParseError::InvalidPassword(PasswordInputError::Empty))
            ));
        }

        #[test]
        fn test_login_command_valid_credentials_returns_typed_command() {
            let command = LoginCommand::parse(" ada ", " secret ").unwrap();

            assert_eq!(command.username().as_str(), "ada");
            assert_eq!(command.password().expose_secret(), " secret ");
        }

        struct FixedUserRepository {
            user: Option<User>,
            unavailable: bool,
        }

        impl UserRepository for FixedUserRepository {
            fn find_by_username(
                &self,
                _username: &Username,
            ) -> Result<Option<User>, UserLookupError> {
                if self.unavailable {
                    Err(UserLookupError::Unavailable)
                } else {
                    Ok(self.user.clone())
                }
            }

            fn save(&self, _user: User) -> Result<(), domain::auth::UserRepositoryError> {
                Ok(())
            }
        }

        struct FixedVerifier {
            result: Result<bool, PasswordVerificationError>,
        }

        impl PasswordVerifier for FixedVerifier {
            fn verify(
                &self,
                _password: &PlaintextPassword,
                _hash: &PasswordHash,
            ) -> Result<bool, PasswordVerificationError> {
                match &self.result {
                    Ok(value) => Ok(*value),
                    Err(_) => Err(PasswordVerificationError::Unavailable),
                }
            }
        }

        struct FailingHasher;

        impl PasswordHasher for FailingHasher {
            fn hash(
                &self,
                _password: &PlaintextPassword,
            ) -> Result<PasswordHash, PasswordHashingError> {
                Err(PasswordHashingError::Unavailable)
            }
        }

        #[test]
        fn test_password_hasher_unavailable_error_is_returned_through_port() {
            let password = PlaintextPassword::parse("correct horse").unwrap();

            assert!(matches!(
                FailingHasher.hash(&password),
                Err(PasswordHashingError::Unavailable)
            ));
        }

        struct FixedIssuer {
            token: Option<OpaqueAccessToken>,
        }

        impl OpaqueTokenIssuer for FixedIssuer {
            fn issue(&self) -> Result<OpaqueAccessToken, TokenIssuanceError> {
                self.token.clone().ok_or(TokenIssuanceError::Unavailable)
            }
        }

        struct RecordingTokenRepository {
            saved: Mutex<Option<AccessTokenRecord>>,
            unavailable: bool,
        }

        impl AccessTokenRepository for RecordingTokenRepository {
            fn save(&self, token: AccessTokenRecord) -> Result<(), AccessTokenRepositoryError> {
                if self.unavailable {
                    return Err(AccessTokenRepositoryError::Unavailable);
                }
                *self.saved.lock().unwrap() = Some(token);
                Ok(())
            }

            fn find(
                &self,
                _token: &OpaqueAccessToken,
            ) -> Result<Option<AccessTokenRecord>, AccessTokenRepositoryError> {
                Ok(self.saved.lock().unwrap().clone())
            }
        }

        fn interactor(
            user: Option<User>,
            verifier_result: Result<bool, PasswordVerificationError>,
            token: Option<OpaqueAccessToken>,
            token_unavailable: bool,
            lookup_unavailable: bool,
        ) -> (LoginInteractor, Arc<RecordingTokenRepository>) {
            let tokens = Arc::new(RecordingTokenRepository {
                saved: Mutex::new(None),
                unavailable: token_unavailable,
            });
            let interactor = LoginInteractor::new(
                Arc::new(FixedUserRepository { user, unavailable: lookup_unavailable }),
                Arc::new(FixedVerifier { result: verifier_result }),
                Arc::new(FixedIssuer { token }),
                tokens.clone(),
            );
            (interactor, tokens)
        }

        #[test]
        fn test_login_interactor_valid_credentials_issues_and_persists_access_token() {
            let token = OpaqueAccessToken::new("issued-token".to_owned()).unwrap();
            let (interactor, tokens) =
                interactor(Some(valid_user()), Ok(true), Some(token.clone()), false, false);

            let result = interactor.execute(valid_command()).unwrap();

            assert_eq!(result.as_str(), "issued-token");
            assert_eq!(
                tokens.saved.lock().unwrap().as_ref().map(|record| record.token().as_str()),
                Some(token.as_str())
            );
        }

        #[test]
        fn test_login_interactor_missing_user_returns_user_not_found_error() {
            let (interactor, _) = interactor(None, Ok(true), None, false, false);

            assert!(matches!(interactor.execute(valid_command()), Err(LoginError::UserNotFound)));
        }

        #[test]
        fn test_login_interactor_wrong_password_returns_invalid_credentials_error() {
            let (interactor, _) = interactor(Some(valid_user()), Ok(false), None, false, false);

            assert!(matches!(
                interactor.execute(valid_command()),
                Err(LoginError::InvalidCredentials)
            ));
        }

        #[test]
        fn test_login_interactor_lookup_failure_returns_lookup_error() {
            let (interactor, _) = interactor(Some(valid_user()), Ok(true), None, false, true);

            assert!(matches!(
                interactor.execute(valid_command()),
                Err(LoginError::Lookup(UserLookupError::Unavailable))
            ));
        }

        #[test]
        fn test_login_interactor_verification_failure_returns_verification_error() {
            let (interactor, _) = interactor(
                Some(valid_user()),
                Err(PasswordVerificationError::Unavailable),
                None,
                false,
                false,
            );

            assert!(matches!(
                interactor.execute(valid_command()),
                Err(LoginError::Verification(PasswordVerificationError::Unavailable))
            ));
        }

        #[test]
        fn test_login_interactor_issuance_failure_returns_token_issuance_error() {
            let (interactor, _) = interactor(Some(valid_user()), Ok(true), None, false, false);

            assert!(matches!(
                interactor.execute(valid_command()),
                Err(LoginError::TokenIssuance(TokenIssuanceError::Unavailable))
            ));
        }

        #[test]
        fn test_login_interactor_persistence_failure_returns_persistence_error() {
            let token = OpaqueAccessToken::new("issued-token".to_owned()).unwrap();
            let (interactor, _) =
                interactor(Some(valid_user()), Ok(true), Some(token), true, false);

            assert!(matches!(
                interactor.execute(valid_command()),
                Err(LoginError::Persistence(AccessTokenRepositoryError::Unavailable))
            ));
        }
    }
}
