<!-- Generated from usecase-types.json — DO NOT EDIT DIRECTLY -->

## Value Objects

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| usecase::auth::PlaintextPassword | value_object | reference | — | 🔵 | 🔵 |

## Error Types

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| usecase::GreetError | error_type | reference | Unavailable | 🔵 | 🔵 |
| usecase::auth::CredentialParseError | error_type | reference | InvalidUsername, InvalidPassword | 🔵 | 🔵 |
| usecase::auth::LoginError | error_type | reference | UserNotFound, InvalidCredentials, Lookup, Verification, TokenIssuance, Persistence | 🔵 | 🔵 |
| usecase::auth::PasswordHashingError | error_type | reference | Unavailable | 🔵 | 🔵 |
| usecase::auth::PasswordInputError | error_type | reference | Empty, TooLong | 🔵 | 🔵 |
| usecase::auth::PasswordVerificationError | error_type | reference | Unavailable | 🔵 | 🔵 |
| usecase::auth::RegisterUserError | error_type | reference | UserAlreadyExists, Lookup, Hashing, Persistence | 🔵 | 🔵 |
| usecase::auth::TokenIssuanceError | error_type | reference | Unavailable | 🔵 | 🔵 |

## Secondary Ports

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| OpaqueTokenIssuer | secondary_port | reference | fn issue(&self) -> Result<domain::auth::OpaqueAccessToken, TokenIssuanceError> | 🔵 | 🔵 |
| PasswordHasher | secondary_port | reference | fn hash(&self, password: &PlaintextPassword) -> Result<domain::auth::PasswordHash, PasswordHashingError> | 🔵 | 🔵 |
| PasswordVerifier | secondary_port | reference | fn verify(&self, password: &PlaintextPassword, hash: &domain::auth::PasswordHash) -> Result<bool, PasswordVerificationError> | 🔵 | 🔵 |
| SalutationProvider | secondary_port | reference | fn salutation(&self) -> Result<String, GreetError> | 🔵 | 🔵 |

## Application Services

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| LoginService | application_service | reference | fn execute(&self, command: LoginCommand) -> Result<IssuedAccessToken, LoginError> | 🔵 | 🔵 |
| RegisterUserService | application_service | reference | fn execute(&self, command: RegisterUserCommand) -> Result<(), RegisterUserError> | 🔵 | 🔵 |

## Use Cases

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| usecase::GreetUser | use_case | reference | — | 🔵 | 🔵 |

## Interactors

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| usecase::auth::LoginInteractor | interactor | reference | — | 🔵 | 🔵 |
| usecase::auth::RegisterUserInteractor | interactor | reference | — | 🔵 | 🔵 |

## DTOs

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| usecase::auth::IssuedAccessToken | dto | reference | — | 🔵 | 🔵 |

## Commands

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| usecase::auth::LoginCommand | command | reference | — | 🔵 | 🔵 |
| usecase::auth::RegisterUserCommand | command | reference | — | 🔵 | 🔵 |

