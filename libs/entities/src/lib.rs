//! Entities layer.
//!
//! The innermost layer of the hexagonal architecture. It holds pure business
//! types and rules and has **no** dependency on any other workspace crate.
//!
//! This template ships a single worked example — the [`Username`] newtype — to
//! demonstrate the "parse, don't validate" pattern: an invariant (non-empty)
//! is enforced once, in a constructor, and encoded in the type thereafter.

use thiserror::Error;

/// Error returned when a [`Username`] fails validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UsernameError {
    /// The provided value was empty or contained only whitespace.
    #[error("username must not be empty")]
    Empty,
}

/// A validated, non-empty username.
///
/// Construct one through [`Username::new`]; the wrapped string is guaranteed to
/// be non-empty and free of surrounding whitespace for the value's lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    /// Creates a `Username`, trimming surrounding whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`UsernameError::Empty`] when the trimmed input is empty.
    pub fn new(raw: impl Into<String>) -> Result<Self, UsernameError> {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(UsernameError::Empty);
        }
        Ok(Self(trimmed))
    }

    /// Returns the username as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_and_accepts_non_empty() {
        let name = Username::new("  ada  ").unwrap();
        assert_eq!(name.as_str(), "ada");
    }

    #[test]
    fn new_rejects_empty_input() {
        assert_eq!(Username::new("   "), Err(UsernameError::Empty));
    }
}

/// Authentication-domain values, aggregates, and replaceable persistence ports.
pub mod auth {
    use thiserror::Error;

    use super::Username;

    const ARGON2ID_PREFIX: &str = "$argon2id$";

    /// Error returned when a stored password hash is not an argon2id encoding.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum PasswordHashError {
        /// The encoded value does not identify an argon2id hash.
        #[error("password hash must use argon2id encoding")]
        InvalidEncoding,
    }

    /// A validated argon2id password-hash representation.
    #[derive(Clone, PartialEq, Eq)]
    pub struct PasswordHash {
        value: String,
    }

    impl PasswordHash {
        /// Creates a password hash from its encoded representation.
        ///
        /// # Errors
        ///
        /// Returns [`PasswordHashError::InvalidEncoding`] when `encoded` is not
        /// a complete PHC argon2id encoding (`$argon2id$v=19$m=…,t=…,p=…$salt$digest`).
        pub fn new(encoded: String) -> Result<Self, PasswordHashError> {
            if !is_complete_argon2id_phc(&encoded) {
                return Err(PasswordHashError::InvalidEncoding);
            }
            Ok(Self { value: encoded })
        }

        /// Returns the encoded password hash.
        #[must_use]
        pub fn as_str(&self) -> &str {
            &self.value
        }

        /// Reports whether this value uses the argon2id encoding.
        #[must_use]
        pub fn is_argon2id(&self) -> bool {
            is_complete_argon2id_phc(&self.value)
        }
    }

    fn is_complete_argon2id_phc(encoded: &str) -> bool {
        if !encoded.starts_with(ARGON2ID_PREFIX) {
            return false;
        }
        let mut fields = encoded.split('$');
        if fields.next() != Some("") || fields.next() != Some("argon2id") {
            return false;
        }
        if fields.next() != Some("v=19") {
            return false;
        }
        let Some(params) = fields.next() else {
            return false;
        };
        let Some(salt) = fields.next() else {
            return false;
        };
        let Some(digest) = fields.next() else {
            return false;
        };
        if fields.next().is_some()
            || !is_base64_unpadded_alphabet(salt)
            || !is_base64_unpadded_alphabet(digest)
        {
            return false;
        }
        parse_phc_params(params).is_some()
    }

    fn is_base64_unpadded_alphabet(value: &str) -> bool {
        // Accept PHC modified Base64 (`.`) and the standard unpadded alphabet (`+/`)
        // used by this workspace's frameworks encoder (`base64ct::Base64Unpadded`).
        !value.is_empty()
            && value.bytes().all(|byte| {
                matches!(
                    byte,
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'.'
                )
            })
    }

    fn parse_phc_params(params: &str) -> Option<(u32, u32, u32)> {
        let mut memory_cost = None;
        let mut time_cost = None;
        let mut parallelism = None;
        for pair in params.split(',') {
            let (name, value) = pair.split_once('=')?;
            let value: u32 = value.parse().ok()?;
            if value == 0 {
                return None;
            }
            match name {
                "m" if memory_cost.is_none() => memory_cost = Some(value),
                "t" if time_cost.is_none() => time_cost = Some(value),
                "p" if parallelism.is_none() => parallelism = Some(value),
                _ => return None,
            }
        }
        Some((memory_cost?, time_cost?, parallelism?))
    }

    /// Error returned when an opaque access token is empty.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum OpaqueAccessTokenError {
        /// The token representation is empty.
        #[error("opaque access token must not be empty")]
        Empty,
    }

    /// A non-empty opaque access-token representation.
    #[derive(Clone, PartialEq, Eq)]
    pub struct OpaqueAccessToken {
        value: String,
    }

    impl OpaqueAccessToken {
        /// Creates an opaque access token from its raw representation.
        ///
        /// # Errors
        ///
        /// Returns [`OpaqueAccessTokenError::Empty`] when `raw` is empty.
        pub fn new(raw: String) -> Result<Self, OpaqueAccessTokenError> {
            if raw.is_empty() {
                return Err(OpaqueAccessTokenError::Empty);
            }
            Ok(Self { value: raw })
        }

        /// Returns the opaque token representation.
        #[must_use]
        pub fn as_str(&self) -> &str {
            &self.value
        }

        /// Reports whether this token has a non-empty representation.
        #[must_use]
        pub fn is_non_empty(&self) -> bool {
            !self.value.is_empty()
        }
    }

    /// A registered user aggregate with a validated username and password hash.
    #[derive(Clone, PartialEq, Eq)]
    pub struct User {
        username: Username,
        password_hash: PasswordHash,
    }

    impl User {
        /// Creates a user aggregate from validated identity and credentials.
        #[must_use]
        pub fn new(username: Username, password_hash: PasswordHash) -> Self {
            Self { username, password_hash }
        }

        /// Returns the user's validated username.
        #[must_use]
        pub fn username(&self) -> &Username {
            &self.username
        }

        /// Returns the user's validated password hash.
        #[must_use]
        pub fn password_hash(&self) -> &PasswordHash {
            &self.password_hash
        }
    }

    /// A persisted opaque access-token record associated with a user.
    #[derive(Clone, PartialEq, Eq)]
    pub struct AccessTokenRecord {
        token: OpaqueAccessToken,
        subject: Username,
    }

    impl AccessTokenRecord {
        /// Creates an access-token record.
        #[must_use]
        pub fn new(token: OpaqueAccessToken, subject: Username) -> Self {
            Self { token, subject }
        }

        /// Returns the record's opaque token.
        #[must_use]
        pub fn token(&self) -> &OpaqueAccessToken {
            &self.token
        }

        /// Returns the username associated with the token.
        #[must_use]
        pub fn subject(&self) -> &Username {
            &self.subject
        }
    }

    /// Failure returned by the user repository boundary.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum UserRepositoryError {
        /// The repository could not complete the operation.
        #[error("user repository is unavailable")]
        Unavailable,
        /// A user with the same username is already stored.
        #[error("user already exists")]
        DuplicateUser,
    }

    /// Failure returned by user-repository lookup operations.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum UserLookupError {
        /// The repository could not complete the lookup.
        #[error("user lookup is unavailable")]
        Unavailable,
    }

    /// Failure returned by the access-token repository boundary.
    #[derive(Debug, Error, PartialEq, Eq)]
    pub enum AccessTokenRepositoryError {
        /// The repository could not complete the operation.
        #[error("access-token repository is unavailable")]
        Unavailable,
    }

    /// Replaceable persistence port for user aggregates.
    pub trait UserRepository: Send + Sync {
        /// Finds a user by username.
        ///
        /// # Errors
        ///
        /// Returns [`UserLookupError::Unavailable`] when the lookup cannot be
        /// completed.
        fn find_by_username(&self, username: &Username) -> Result<Option<User>, UserLookupError>;

        /// Persists a user aggregate.
        ///
        /// # Errors
        ///
        /// Returns [`UserRepositoryError::DuplicateUser`] when the username
        /// is already stored, or [`UserRepositoryError::Unavailable`] when the
        /// repository cannot complete the operation.
        fn save(&self, user: User) -> Result<(), UserRepositoryError>;
    }

    /// Replaceable persistence port for opaque access-token records.
    pub trait AccessTokenRepository: Send + Sync {
        /// Persists an access-token record.
        ///
        /// # Errors
        ///
        /// Returns [`AccessTokenRepositoryError::Unavailable`] when the
        /// repository cannot complete the operation.
        fn save(&self, token: AccessTokenRecord) -> Result<(), AccessTokenRepositoryError>;

        /// Finds an access-token record by token.
        ///
        /// # Errors
        ///
        /// Returns [`AccessTokenRepositoryError::Unavailable`] when the
        /// repository cannot complete the operation.
        fn find(
            &self,
            token: &OpaqueAccessToken,
        ) -> Result<Option<AccessTokenRecord>, AccessTokenRepositoryError>;
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use super::*;

        fn valid_hash() -> PasswordHash {
            PasswordHash::new("$argon2id$v=19$m=19456,t=2,p=1$salt$hash".to_owned()).unwrap()
        }

        fn valid_username() -> Username {
            Username::new("ada").unwrap()
        }

        #[test]
        fn test_opaque_access_token_empty_input_returns_empty_error() {
            assert!(matches!(
                OpaqueAccessToken::new(String::new()),
                Err(OpaqueAccessTokenError::Empty)
            ));

            let token = OpaqueAccessToken::new("opaque".to_owned()).unwrap();
            assert!(token.is_non_empty());
            assert_eq!(token.as_str(), "opaque");
        }

        #[test]
        fn test_password_hash_non_argon2id_input_returns_invalid_encoding_error() {
            assert!(matches!(
                PasswordHash::new("plaintext".to_owned()),
                Err(PasswordHashError::InvalidEncoding)
            ));
            assert!(matches!(
                PasswordHash::new("$argon2i$v=19$invalid".to_owned()),
                Err(PasswordHashError::InvalidEncoding)
            ));
            assert!(matches!(
                PasswordHash::new("$argon2id$".to_owned()),
                Err(PasswordHashError::InvalidEncoding)
            ));
            assert!(matches!(
                PasswordHash::new("$argon2id$garbage".to_owned()),
                Err(PasswordHashError::InvalidEncoding)
            ));
            assert!(matches!(
                PasswordHash::new("$argon2id$v=19$m=19456,t=2,p=1$salt".to_owned()),
                Err(PasswordHashError::InvalidEncoding)
            ));
            assert!(matches!(
                PasswordHash::new("$argon2id$v=19$m=19456,t=2,p=1$%%%$%%%".to_owned()),
                Err(PasswordHashError::InvalidEncoding)
            ));
            assert!(matches!(
                PasswordHash::new("$argon2id$v=19$m=0,t=2,p=1$salt$hash".to_owned()),
                Err(PasswordHashError::InvalidEncoding)
            ));
            assert!(
                PasswordHash::new(
                    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$dmFsaWQuYmFzZTY0LmRpZ2VzdC4".to_owned()
                )
                .is_ok()
            );

            let hash = valid_hash();
            assert!(hash.is_argon2id());
        }

        #[test]
        fn test_user_aggregate_construction_preserves_validated_values() {
            let user = User::new(valid_username(), valid_hash());

            assert_eq!(user.username().as_str(), "ada");
            assert!(user.password_hash().is_argon2id());
        }

        #[test]
        fn test_access_token_record_construction_preserves_token_and_subject() {
            let record = AccessTokenRecord::new(
                OpaqueAccessToken::new("opaque".to_owned()).unwrap(),
                valid_username(),
            );

            assert_eq!(record.token().as_str(), "opaque");
            assert_eq!(record.subject().as_str(), "ada");
        }

        struct FakeUserRepository {
            user: User,
        }

        impl UserRepository for FakeUserRepository {
            fn find_by_username(
                &self,
                username: &Username,
            ) -> Result<Option<User>, UserLookupError> {
                if self.user.username() == username {
                    Ok(Some(self.user.clone()))
                } else {
                    Ok(None)
                }
            }

            fn save(&self, _user: User) -> Result<(), UserRepositoryError> {
                Ok(())
            }
        }

        #[test]
        fn test_user_repository_find_by_username_returns_matching_user() {
            let user = User::new(valid_username(), valid_hash());
            let repository = FakeUserRepository { user: user.clone() };

            let found = repository.find_by_username(user.username()).unwrap();
            assert!(found.is_some());
            assert_eq!(found.as_ref().map(User::username), Some(user.username()));
        }

        #[test]
        fn test_user_repository_save_accepts_user_through_port() {
            let repository = FakeUserRepository { user: User::new(valid_username(), valid_hash()) };

            assert_eq!(repository.save(repository.user.clone()), Ok(()));
        }

        struct FakeAccessTokenRepository {
            record: AccessTokenRecord,
        }

        impl AccessTokenRepository for FakeAccessTokenRepository {
            fn save(&self, _token: AccessTokenRecord) -> Result<(), AccessTokenRepositoryError> {
                Ok(())
            }

            fn find(
                &self,
                token: &OpaqueAccessToken,
            ) -> Result<Option<AccessTokenRecord>, AccessTokenRepositoryError> {
                if self.record.token() == token { Ok(Some(self.record.clone())) } else { Ok(None) }
            }
        }

        #[test]
        fn test_access_token_repository_save_accepts_record_through_port() {
            let repository = FakeAccessTokenRepository {
                record: AccessTokenRecord::new(
                    OpaqueAccessToken::new("opaque".to_owned()).unwrap(),
                    valid_username(),
                ),
            };

            assert_eq!(repository.save(repository.record.clone()), Ok(()));
        }

        #[test]
        fn test_access_token_repository_find_returns_matching_record() {
            let repository = FakeAccessTokenRepository {
                record: AccessTokenRecord::new(
                    OpaqueAccessToken::new("opaque".to_owned()).unwrap(),
                    valid_username(),
                ),
            };
            let token = repository.record.token().clone();

            let found = repository.find(&token).unwrap();
            assert!(found.is_some());
            assert_eq!(found.as_ref().map(|record| record.token().as_str()), Some(token.as_str()));
        }
    }
}
