<!-- Generated from usecase-types.json — DO NOT EDIT DIRECTLY -->

## Value Objects

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| PlaintextPassword | value_object | add | — | 🔵 | 🔵 |

## Error Types

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| CredentialParseError | error_type | add | InvalidUsername, InvalidPassword | 🔵 | 🔵 |
| LoginError | error_type | add | UserNotFound, InvalidCredentials, Lookup, Verification, TokenIssuance, Persistence | 🔵 | 🔵 |
| PasswordHashingError | error_type | add | Unavailable | 🔵 | 🔵 |
| PasswordInputError | error_type | add | Empty, TooLong | 🔵 | 🔵 |
| PasswordVerificationError | error_type | add | Unavailable | 🔵 | 🔵 |
| RegisterUserError | error_type | add | UserAlreadyExists, Lookup, Hashing, Persistence | 🔵 | 🔵 |
| TokenIssuanceError | error_type | add | Unavailable | 🔵 | 🔵 |

## Secondary Ports

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| OpaqueTokenIssuer | secondary_port | add | fn issue(&self) -> Result<domain::auth::OpaqueAccessToken, TokenIssuanceError> | 🔵 | 🔵 |
| PasswordHasher | secondary_port | add | fn hash(&self, password: &PlaintextPassword) -> Result<domain::auth::PasswordHash, PasswordHashingError> | 🔵 | 🔵 |
| PasswordVerifier | secondary_port | add | fn verify(&self, password: &PlaintextPassword, hash: &domain::auth::PasswordHash) -> Result<bool, PasswordVerificationError> | 🔵 | 🔵 |

## Application Services

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| LoginService | application_service | add | fn execute(&self, command: LoginCommand) -> Result<IssuedAccessToken, LoginError> | 🔵 | 🔵 |
| RegisterUserService | application_service | add | fn execute(&self, command: RegisterUserCommand) -> Result<(), RegisterUserError> | 🔵 | 🔵 |

## Interactors

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| LoginInteractor | interactor | add | — | 🔵 | 🔵 |
| RegisterUserInteractor | interactor | add | — | 🔵 | 🔵 |

## DTOs

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| IssuedAccessToken | dto | add | — | 🔵 | 🔵 |

## Commands

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| LoginCommand | command | add | — | 🔵 | 🔵 |
| RegisterUserCommand | command | add | — | 🔵 | 🔵 |

