<!-- Generated from use_cases-types.json — DO NOT EDIT DIRECTLY -->

## Value Objects

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| use_cases::auth::PlaintextPassword | value_object | reference | — | 🔵 | 🔵 |

## Error Types

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| use_cases::GreetError | error_type | reference | Unavailable | 🔵 | 🔵 |
| use_cases::auth::CredentialParseError | error_type | reference | InvalidUsername, InvalidPassword | 🔵 | 🔵 |
| use_cases::auth::LoginError | error_type | reference | UserNotFound, InvalidCredentials, Lookup, Verification, TokenIssuance, Persistence | 🔵 | 🔵 |
| use_cases::auth::PasswordHashingError | error_type | reference | Unavailable | 🔵 | 🔵 |
| use_cases::auth::PasswordInputError | error_type | reference | Empty, TooLong | 🔵 | 🔵 |
| use_cases::auth::PasswordVerificationError | error_type | reference | Unavailable | 🔵 | 🔵 |
| use_cases::auth::RegisterUserError | error_type | reference | UserAlreadyExists, Lookup, Hashing, Persistence | 🔵 | 🔵 |
| use_cases::auth::TokenIssuanceError | error_type | reference | Unavailable | 🔵 | 🔵 |

## Secondary Ports

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| OpaqueTokenIssuer | secondary_port | reference | fn issue(&self) -> Result<entities::auth::OpaqueAccessToken, TokenIssuanceError> | 🔵 | 🔵 |
| PasswordHasher | secondary_port | reference | fn hash(&self, password: &PlaintextPassword) -> Result<entities::auth::PasswordHash, PasswordHashingError> | 🔵 | 🔵 |
| PasswordVerifier | secondary_port | reference | fn verify(&self, password: &PlaintextPassword, hash: &entities::auth::PasswordHash) -> Result<bool, PasswordVerificationError> | 🔵 | 🔵 |
| SalutationProvider | secondary_port | reference | fn salutation(&self) -> Result<String, GreetError> | 🔵 | 🔵 |

## Application Services

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| LoginService | application_service | reference | fn execute(&self, command: LoginCommand) -> Result<IssuedAccessToken, LoginError> | 🔵 | 🔵 |
| RegisterUserService | application_service | reference | fn execute(&self, command: RegisterUserCommand) -> Result<(), RegisterUserError> | 🔵 | 🔵 |

## Use Cases

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| use_cases::GreetUser | use_case | reference | — | 🔵 | 🔵 |

## Interactors

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| use_cases::auth::LoginInteractor | interactor | reference | — | 🔵 | 🔵 |
| use_cases::auth::RegisterUserInteractor | interactor | reference | — | 🔵 | 🔵 |

## DTOs

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| use_cases::auth::IssuedAccessToken | dto | reference | — | 🔵 | 🔵 |

## Commands

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| use_cases::auth::LoginCommand | command | reference | — | 🔵 | 🔵 |
| use_cases::auth::RegisterUserCommand | command | reference | — | 🔵 | 🔵 |

