<!-- Generated from frameworks-types.json — DO NOT EDIT DIRECTLY -->

## Enums

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| frameworks::http::HttpServerOutcome | enum | add | Stopped, Failed | 🔵 | 🔵 |

## Secondary Adapters

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| frameworks::StaticSalutation | secondary_adapter | reference | — | 🔵 | 🔵 |
| frameworks::auth::Argon2PasswordAdapter | secondary_adapter | reference | — | 🔵 | 🔵 |
| frameworks::auth::InMemoryAccessTokenRepository | secondary_adapter | reference | — | 🔵 | 🔵 |
| frameworks::auth::InMemoryUserRepository | secondary_adapter | reference | — | 🔵 | 🔵 |
| frameworks::auth::RandomOpaqueTokenIssuer | secondary_adapter | reference | — | 🔵 | 🔵 |

## Primary Adapters

| Name | Kind | Action | Details | Signal | Cat-Spec |
|------|------|--------|---------|--------|----------|
| frameworks::http::HttpServer | primary_adapter | add | — | 🔵 | 🔵 |

