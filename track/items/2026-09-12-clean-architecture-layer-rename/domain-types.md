<!-- Generated from domain-types.json — DO NOT EDIT DIRECTLY -->

## Value Objects

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| domain::Username | value_object | reference | — | 🔵 | 🔵 |
| domain::auth::OpaqueAccessToken | value_object | reference | — | 🔵 | 🔵 |
| domain::auth::PasswordHash | value_object | reference | — | 🔵 | 🔵 |

## Error Types

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| domain::UsernameError | error_type | reference | Empty | 🔵 | 🔵 |
| domain::auth::AccessTokenRepositoryError | error_type | reference | Unavailable | 🔵 | 🔵 |
| domain::auth::OpaqueAccessTokenError | error_type | reference | Empty | 🔵 | 🔵 |
| domain::auth::PasswordHashError | error_type | reference | InvalidEncoding | 🔵 | 🔵 |
| domain::auth::UserLookupError | error_type | reference | Unavailable | 🔵 | 🔵 |
| domain::auth::UserRepositoryError | error_type | reference | Unavailable, DuplicateUser | 🔵 | 🔵 |

## Aggregate Roots

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| domain::auth::AccessTokenRecord | aggregate_root | reference | — | 🔵 | 🔵 |
| domain::auth::User | aggregate_root | reference | — | 🔵 | 🔵 |

## Repositories

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| AccessTokenRepository | repository | reference | fn save(&self, token: AccessTokenRecord) -> Result<(), AccessTokenRepositoryError>, fn find(&self, token: &OpaqueAccessToken) -> Result<Option<AccessTokenRecord>, AccessTokenRepositoryError> | 🔵 | 🔵 |
| UserRepository | repository | reference | fn find_by_username(&self, username: &Username) -> Result<Option<User>, UserLookupError>, fn save(&self, user: User) -> Result<(), UserRepositoryError> | 🔵 | 🔵 |

