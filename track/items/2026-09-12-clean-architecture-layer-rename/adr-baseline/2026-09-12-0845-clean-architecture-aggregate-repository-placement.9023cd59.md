---
adr_id: "2026-09-12-0845-clean-architecture-aggregate-repository-placement"
decisions:
  - id: D1
    user_decision_ref: "chat_segment:sotohe-trial-adr2pr:2026-09-12:clean-architecture-layer-rename:repository-binding"
    candidate_selection: "from:[move-repository-ports-to-use_cases, keep-aggregate-repository-in-entities] chose:keep-aggregate-repository-in-entities"
    status: proposed
---
# Aggregate Repository ports remain in entities

## Context

ADR `2026-09-12-0139-clean-architecture-layer-rename` D3 places ports/interfaces in `use_cases`. Standing catalogue-lint and `type-designer-kind-selection` keep the `Repository` role on the innermost layer with its aggregates. During this track's binding adjudication, relocating `UserRepository` / `AccessTokenRepository` into `use_cases` (or widening `Repository` `permitted_layers`) was rejected.

## Decision

### D1: Aggregate Repository ports stay in entities

For this Clean Architecture rename:

1. Application ports (use-case facing secondary ports such as hashing, verification, token issuance, and application services) live in `use_cases`.
2. Aggregate `Repository` ports remain in `entities` beside their aggregates (`UserRepository`, `AccessTokenRepository`).
3. Do not widen `Repository` `permitted_layers` beyond `entities`.

This refines D3 of the parent rename ADR: “ports in use_cases” means application ports, not aggregate persistence ports.

## Rejected Alternatives

- Move `UserRepository` / `AccessTokenRepository` into `use_cases` and widen the Repository lint: rejected by binding adjudication; conflicts with the innermost-Repository convention.
- Leave AC-003 wording as “all ports in use_cases” while encoding an entities-only Repository lint: rejected — the PR review correctly treated that as an inconsistent acceptance criterion.

## Consequences

- Spec AC-003 and catalogue-lint agree: application ports in `use_cases`, aggregate repositories in `entities`.
- Frameworks continue to implement repository ports defined next to aggregates.

## Related

- Refines: `knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3`

- `knowledge/adr/2026-09-12-0139-clean-architecture-layer-rename.md#D3`
- `.harness/catalogue-lint/config.json` Repository `KindLayerConstraint`
