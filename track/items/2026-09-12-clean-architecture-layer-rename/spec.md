<!-- Generated from spec.json — DO NOT EDIT DIRECTLY -->
---
version: "1.0.0"
signals: { blue: 17, yellow: 0, red: 0 }
---

# クリーンアーキテクチャ層へのリネーム

## Goal

- [GO-001] Migrate the existing consumer workspace to the textbook Clean Architecture layer vocabulary and placement while preserving its authentication product behavior. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D1, knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3]

## Scope

### In Scope
- [IN-001] Rename the existing six workspace crates and their Rust crate identifiers to the decided Clean Architecture layer names. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D1] [tasks: T001, T002, T003, T004, T005, T006, T007, T008]
- [IN-002] Maintain the decided dependency ring and update its enforcement configuration for the renamed crates. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D2] [tasks: T001, T004, T005, T006, T007, T008]
- [IN-003] Relocate the existing ports, HTTP delivery components, infrastructure implementations, composition root, and binary entry point to their decided Clean Architecture boundaries. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3] [tasks: T002, T003, T004, T005, T009, T006, T007, T008]
- [IN-004] Preserve the existing HTTP registration and login behavior through the internal migration. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3, knowledge/adr/2026-09-10-1238-minimal-auth-registration-capability.md#D1] [tasks: T003, T005, T009, T006, T007, T008]

### Out of Scope
- [OS-001] Changing the existing registration or login HTTP contract or authentication behavior is out of scope. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3] [tasks: T005, T009, T008]
- [OS-002] Adding a new authentication product capability is out of scope. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3] [tasks: T005, T009, T008]
- [OS-003] Collapsing composition and the binary entry point into a four-layer architecture is out of scope. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3] [tasks: T006, T007, T008]

## Constraints
- [CN-001] The layer vocabulary is entities, use_cases, interface_adapters, frameworks, web_composition, and web, mapped to the decided library and application paths. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D1] [tasks: T001, T002, T003, T004, T005, T006, T007, T008]
- [CN-002] entities has no workspace-layer dependency; use_cases depends only on entities; interface_adapters depends only on use_cases; frameworks depends only on entities and use_cases; web_composition depends only on entities, use_cases, frameworks, and interface_adapters; and web depends only on web_composition and interface_adapters. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D2] [tasks: T001, T002, T003, T004, T005, T006, T007, T008]
- [CN-003] architecture-rules.json and deny.toml remain synchronized with the renamed crates and the decided dependency direction. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D2] [tasks: T001, T004, T005, T008]
- [CN-004] Internal relocation follows the decided textbook Clean Architecture placement boundaries without adding a product capability. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3] [tasks: T002, T003, T004, T005, T009, T006, T007, T008]
- [CN-005] The migration preserves the existing REST authentication surface and its opaque-token behavior. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3, knowledge/adr/2026-09-10-1126-minimal-auth-api.md#D1] [tasks: T003, T005, T009, T006, T007, T008]

## Acceptance Criteria
- [ ] [AC-001] The workspace exposes the six Clean Architecture crates at the decided paths and with the decided Rust crate identifiers. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D1] [tasks: T001, T002, T003, T004, T005, T006, T007, T008]
- [ ] [AC-002] Declared workspace dependencies and their architecture-rule and dependency-denial enforcement admit only the decided dependency ring. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D2] [tasks: T001, T002, T003, T004, T005, T006, T007, T008]
- [ ] [AC-003] Ports are located in use_cases; HTTP controllers and DTO conversion are located in interface_adapters; persistence, cryptography, and axum server wiring are located in frameworks; assembly is located only in web_composition; and web remains a thin binary entry point. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3] [tasks: T003, T004, T005, T009, T006, T007, T008]
- [ ] [AC-004] The existing registration and login HTTP contracts and authentication behavior, including empty-password rejection, remain unchanged after the migration. [adr: knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3] [tasks: T003, T005, T009, T006, T007, T008]

## Related Conventions (Required Reading)
- knowledge/conventions/coding-principles.md#purpose
- knowledge/conventions/prefer-type-safe-abstractions.md#rule

## Signal Summary

### Stage 1: Spec Signals
🔵 17  🟡 0  🔴 0

