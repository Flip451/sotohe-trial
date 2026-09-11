<!-- Generated from metadata.json + impl-plan.json — DO NOT EDIT DIRECTLY -->
# 最小認証API

## Summary

GO-001 — implement the domain-foundation, application-services, secondary-adapters, http-delivery, and composition sections through T001, T002, T003, T004, T005, T006, T007, T008, and T009.

## Tasks (7/9 resolved)

### domain-foundation — Domain foundation

> T001 — domain catalogue entries and tests (IN-001, IN-002, CN-001, CN-002, AC-001, AC-002, AC-003).

- [x] **T001**: Implement the domain entries `PasswordHash`, `PasswordHashError`, `OpaqueAccessToken`, `OpaqueAccessTokenError`, `User`, `AccessTokenRecord`, `UserRepository`, `AccessTokenRepository`, `UserRepositoryError`, `AccessTokenRepositoryError`, and `UserLookupError`, plus focused boundary and contract tests (IN-001, IN-002, CN-001, CN-002, AC-001, AC-002, AC-003). (`6062f10fb38b2fa249d1c7706b9de6369518c530`)

### application-services — Application services

> T002, T003, T004 — usecase catalogue entries and tests (IN-001, IN-002, CN-001, CN-002, CN-005, AC-001, AC-002, AC-003).

- [x] **T002**: Implement the usecase entries `PlaintextPassword`, `PasswordInputError`, `CredentialParseError`, `RegisterUserCommand`, `LoginCommand`, `PasswordHashingError`, `PasswordVerificationError`, `TokenIssuanceError`, `PasswordHasher`, `PasswordVerifier`, and `OpaqueTokenIssuer`, plus focused boundary tests (IN-001, IN-002, CN-001, CN-005, AC-001, AC-002, AC-003). (`6062f10fb38b2fa249d1c7706b9de6369518c530`)
- [x] **T003**: Implement the usecase entries `RegisterUserService`, `RegisterUserInteractor`, and `RegisterUserError`, plus result tests (IN-001, CN-001, CN-002, CN-005, AC-001, AC-003). (`21152e95f47e480cefee876b713829f2d023b28a`)
- [x] **T004**: Implement the usecase entries `LoginService`, `LoginInteractor`, `LoginError`, and `IssuedAccessToken`, plus result tests (IN-002, CN-001, CN-002, CN-005, AC-002). (`6062f10fb38b2fa249d1c7706b9de6369518c530`)

### secondary-adapters — Secondary adapters

> T005 — infrastructure catalogue entries and tests (IN-001, IN-002, CN-001, CN-002, AC-001, AC-002, AC-003).

- [x] **T005**: Implement the infrastructure entries `InMemoryUserRepository`, `InMemoryAccessTokenRepository`, `Argon2PasswordAdapter`, and `RandomOpaqueTokenIssuer`, plus repository and port-conformance tests (IN-001, IN-002, CN-001, CN-002, AC-001, AC-002, AC-003). (`6062f10fb38b2fa249d1c7706b9de6369518c530`)

### http-delivery — HTTP delivery

> T006 — shared cli_driver HTTP foundation and tests (IN-001, IN-002, CN-003, CN-004).
> T008 — registration request, route wiring, and tests (IN-001, CN-003, CN-004, CN-005, AC-001).
> T009 — login request, token response, route wiring, and tests (IN-002, CN-003, CN-004, CN-005, AC-002).

- [x] **T006**: Implement the cli_driver entries `AuthHttpApi` and `HttpServerOutcome` as the shared axum HTTP foundation, plus focused server lifecycle, JSON rejection, and HTTP outcome tests (IN-001, IN-002, CN-003, CN-004). (`21152e95f47e480cefee876b713829f2d023b28a`)
- [x] **T008**: Implement the cli_driver entry `RegisterRequest` and wire the registration route into `AuthHttpApi`, plus focused registration request and HTTP behavior tests (IN-001, CN-003, CN-004, CN-005, AC-001).
- [ ] **T009**: Implement the cli_driver entries `LoginRequest` and `AccessTokenResponse` and wire the login/token route into `AuthHttpApi`, plus focused login request, token response, and HTTP behavior tests (IN-002, CN-003, CN-004, CN-005, AC-002).

### composition — Composition

> T007 — cli_composition catalogue entry and tests (IN-001, IN-002, CN-002, CN-003, AC-001, AC-002).

- [ ] **T007**: Implement the cli_composition entry `AuthCompositionRoot`, plus focused wiring tests (IN-001, IN-002, CN-002, CN-003, AC-001, AC-002).
