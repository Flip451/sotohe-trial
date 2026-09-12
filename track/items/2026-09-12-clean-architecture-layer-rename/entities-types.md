<!-- Generated from entities-types.json — DO NOT EDIT DIRECTLY -->

## Value Objects

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| entities::Username | value_object | reference | — | 🔵 | 🔵 |
| entities::auth::OpaqueAccessToken | value_object | reference | — | 🔵 | 🔵 |
| entities::auth::PasswordHash | value_object | reference | — | 🔵 | 🔵 |

## Error Types

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| entities::UsernameError | error_type | reference | Empty | 🔵 | 🔵 |
| entities::auth::AccessTokenRepositoryError | error_type | reference | Unavailable | 🔵 | 🔵 |
| entities::auth::OpaqueAccessTokenError | error_type | reference | Empty | 🔵 | 🔵 |
| entities::auth::PasswordHashError | error_type | reference | InvalidEncoding | 🔵 | 🔵 |
| entities::auth::UserLookupError | error_type | reference | Unavailable | 🔵 | 🔵 |
| entities::auth::UserRepositoryError | error_type | reference | Unavailable, DuplicateUser | 🔵 | 🔵 |

## Aggregate Roots

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| entities::auth::AccessTokenRecord | aggregate_root | reference | — | 🔵 | 🔵 |
| entities::auth::User | aggregate_root | reference | — | 🔵 | 🔵 |

## Repositories

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| AccessTokenRepository | repository | reference | fn save(&self, token: AccessTokenRecord) -> Result<(), AccessTokenRepositoryError>, fn find(&self, token: &OpaqueAccessToken) -> Result<Option<AccessTokenRecord>, AccessTokenRepositoryError> | 🔵 | 🔵 |
| UserRepository | repository | reference | fn find_by_username(&self, username: &Username) -> Result<Option<User>, UserLookupError>, fn save(&self, user: User) -> Result<(), UserRepositoryError> | 🔵 | 🔵 |

