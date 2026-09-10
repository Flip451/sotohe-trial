<!-- Generated from domain-types.json — DO NOT EDIT DIRECTLY -->

## Value Objects

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| OpaqueAccessToken | value_object | add | — | 🟡 | 🔵 |
| PasswordHash | value_object | add | — | 🟡 | 🔵 |
| domain::Username | value_object | reference | — | 🔵 | 🔵 |

## Error Types

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| AccessTokenRepositoryError | error_type | add | Unavailable | 🟡 | 🔵 |
| OpaqueAccessTokenError | error_type | add | Empty | 🟡 | 🔵 |
| PasswordHashError | error_type | add | InvalidEncoding | 🟡 | 🔵 |
| UserLookupError | error_type | add | Unavailable | 🟡 | 🔵 |
| UserRepositoryError | error_type | add | Unavailable, DuplicateUser | 🟡 | 🔵 |
| domain::UsernameError | error_type | reference | Empty | 🔵 | 🔵 |

## Aggregate Roots

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| AccessTokenRecord | aggregate_root | add | — | 🟡 | 🔵 |
| User | aggregate_root | add | — | 🟡 | 🔵 |

## Repositories

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| AccessTokenRepository | repository | add | fn save(&self, token: AccessTokenRecord) -> Result<(), AccessTokenRepositoryError>, fn find(&self, token: &OpaqueAccessToken) -> Result<Option<AccessTokenRecord>, AccessTokenRepositoryError> | 🟡 | 🔵 |
| UserRepository | repository | add | fn find_by_username(&self, username: &Username) -> Result<Option<User>, UserLookupError>, fn save(&self, user: User) -> Result<(), UserRepositoryError> | 🟡 | 🔵 |

