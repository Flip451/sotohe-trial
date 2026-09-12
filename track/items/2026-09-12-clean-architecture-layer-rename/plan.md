<!-- Generated from metadata.json + impl-plan.json — DO NOT EDIT DIRECTLY -->
# クリーンアーキテクチャ層へのリネーム

## Summary

GO-001 — migrate enforcement, all six crates, placement boundaries, documentation, review routing, and reference catalogues through T001–T009 while preserving registration and login behavior.

## Tasks (0/9 resolved)

### enforcement-first — Enforcement first

> T001 — update workspace, architecture-rule, deny, Makefile, catalogue-lint, preset, and fixture configuration for the renamed layer ids and dependency permissions (IN-001, IN-002, CN-001, CN-002, CN-003, AC-001, AC-002).

- [~] **T001**: Update root workspace membership, architecture-rules.json layer ids/paths/dependency permissions, deny.toml wrappers, affected Makefile.toml task names, .harness/catalogue-lint/config.json, presets/ddd-strict.json, and focused enforcement fixtures (IN-001, IN-002, CN-001, CN-002, CN-003, AC-001, AC-002).

### crate-migration — Clean Architecture crate migration

> T002–T007 and T009 — rename and relocate the six implementation crates; update references and wiring (IN-001, IN-002, IN-003, IN-004, OS-001, OS-002, OS-003, CN-001, CN-002, CN-003, CN-004, CN-005, AC-001, AC-002, AC-003, AC-004).

- [~] **T002**: Move libs/domain to libs/entities, rename the Rust crate identifier to entities, and update internal references and tests (IN-001, IN-003, CN-001, CN-002, CN-004, AC-001, AC-002).
- [~] **T003**: Move libs/usecase to libs/use_cases, rename the Rust crate identifier to use_cases, redirect crate references to entities, and update use-case references and tests (IN-001, IN-003, IN-004, CN-001, CN-002, CN-004, CN-005, AC-001, AC-002, AC-003, AC-004).
- [~] **T004**: Move libs/infrastructure to libs/frameworks, rename the Rust crate identifier to frameworks, update dependencies to entities and use_cases, and update references and tests (IN-001, IN-002, IN-003, CN-001, CN-002, CN-003, CN-004, AC-001, AC-002, AC-003).
- [~] **T005**: Move apps/cli-driver to libs/interface_adapters with Rust crate identifier interface_adapters, and update HTTP controller/DTO references and focused tests (IN-001, IN-002, IN-003, IN-004, OS-001, OS-002, CN-001, CN-002, CN-003, CN-004, CN-005, AC-001, AC-002, AC-003, AC-004).
- [~] **T009**: Move axum server lifecycle/wiring from libs/interface_adapters into libs/frameworks and update focused lifecycle tests (IN-003, IN-004, OS-001, OS-002, CN-004, CN-005, AC-003, AC-004).
- [~] **T006**: Move apps/cli-composition to apps/web-composition with Rust crate identifier web_composition, and update dependency declarations, references, and composition-root tests (IN-001, IN-002, IN-003, IN-004, OS-003, CN-001, CN-002, CN-004, CN-005, AC-001, AC-002, AC-003, AC-004).
- [~] **T007**: Move apps/cli to apps/web with Rust crate identifier web, update dependency declarations, and update main to invoke web_composition and interface_adapters (IN-001, IN-002, IN-003, IN-004, OS-003, CN-001, CN-002, CN-004, CN-005, AC-001, AC-002, AC-003, AC-004).

### documentation-and-gates — Documentation, metadata, and terminal gates

> T008 — update architecture documentation, review-scope routing, and reference catalogue filenames/layer ids after enforcement and source migration, then run the required repository gates.

- [~] **T008**: After enforcement and crate migration, update architecture-facing documentation and review-scope group ids/patterns/briefing references, rename the six reference catalogue files and their layer/crate ids plus the TDDD feature declaration to the new vocabulary, preserve catalogue-lint config/preset structural equality, and validate the completed migration with formatting, check-layers, verify-arch-docs, cargo deny, and cargo make ci (IN-001, IN-002, IN-003, IN-004, OS-001, OS-002, OS-003, CN-001, CN-002, CN-003, CN-004, CN-005, AC-001, AC-002, AC-003, AC-004).
