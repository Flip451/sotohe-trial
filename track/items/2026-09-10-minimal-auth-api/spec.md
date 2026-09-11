<!-- Generated from spec.json — DO NOT EDIT DIRECTLY -->
---
version: "1.0.0"
signals: { blue: 13, yellow: 0, red: 0 }
---

# 最小認証API

## Goal

- [GO-001] Issue opaque access tokens after successful authentication for client proof. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D1]
- [GO-002] Expose user registration as an HTTP API capability. [adr: knowledge/adr/2026-09-10-1238-minimal-auth-registration-capability.md#D1]

## Scope

### In Scope
- [IN-001] Register a user through the API. [adr: knowledge/adr/2026-09-10-1238-minimal-auth-registration-capability.md#D1] [tasks: T001, T002, T003, T005, T006, T008, T007]
- [IN-002] Authenticate a user login attempt and issue an access token after successful authentication. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D1] [tasks: T001, T002, T004, T005, T006, T009, T007]

### Out of Scope
- [OS-001] Refresh tokens are not provided. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D1] [tasks: T004, T009]

## Constraints
- [CN-001] Passwords are never persisted in plaintext and are stored using argon2id hashing. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D2] [tasks: T001, T002, T003, T004, T005]
- [CN-002] User and token persistence uses an in-memory implementation behind a repository port so it can be replaced later. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D3] [tasks: T001, T003, T004, T005, T007]
- [CN-003] The HTTP surface is an axum REST API. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D4] [tasks: T006, T008, T009, T007]
- [CN-004] REST API requests use UTF-8 encoded JSON bodies. After UTF-8 decoding, JSON string values are used as received without Unicode normalization. A body with invalid UTF-8 or malformed JSON is rejected with HTTP 400, and a request with a media type other than application/json is rejected with HTTP 415. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D4] [tasks: T006, T008, T009]
- [CN-005] At the application boundary, registration and login reject a request whose presented password is empty. [adr: knowledge/adr/2026-09-10-1716-minimal-auth-empty-password-rejection.md#D1] [tasks: T002, T003, T004, T008, T009]

## Acceptance Criteria
- [ ] [AC-001] A user can be registered through the REST API. [adr: knowledge/adr/2026-09-10-1238-minimal-auth-registration-capability.md#D1] [tasks: T001, T002, T003, T005, T008, T007]
- [ ] [AC-002] Successful login through the REST API issues an opaque access token for client proof after authentication. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D1] [tasks: T001, T002, T004, T005, T009, T007]
- [ ] [AC-003] A registered user's password is retained only as an argon2id hash, never in plaintext. [adr: knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D2] [tasks: T001, T002, T003, T005]

## Related Conventions (Required Reading)
- knowledge/conventions/coding-principles.md#purpose
- knowledge/conventions/prefer-type-safe-abstractions.md#rule

## Signal Summary

### Stage 1: Spec Signals
🔵 13  🟡 0  🔴 0

