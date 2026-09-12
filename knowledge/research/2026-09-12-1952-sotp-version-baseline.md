# Version Baseline: sotp pin bump to sotp-v0.1.2 (2026-09-12)

## Context

Track `2026-09-12-clean-architecture-layer-rename` renamed TDDD layers to
clean-architecture ids (`entities`, `use_cases`, `interface_adapters`,
`frameworks`, `web`, `web_composition`). Remote CI failed on
`bin/sotp task-contract coverage` under pinned `sotp-v0.1.0` because that
release hardcodes the previous template layer set
(`domain` / `usecase` / `infrastructure` / `cli_driver` / `cli` /
`cli_composition`) as the "canonical TDDD layers".

## Candidate tags

| Tag | Commit | Notes |
|-----|--------|-------|
| `sotp-v0.1.0` (previous pin) | template export baseline | Hardcoded `CANONICAL_LAYERS` + hardcoded test-obligation catalogue filenames. Incompatible with architecture-customizer renames. |
| `sotp-v0.1.1` | `2606d3e9` | Resolves task-contract canonical layers from `architecture-rules.json` (`tddd.enabled`) via `TdddLayerBindingsPort`. |
| `sotp-v0.1.2` (chosen) | `369c8c29` | Follow-up: `test-obligation` default catalogue paths also read `catalogue_file` from `architecture-rules.json` (with the old template set as fixture fallback). |

Source: https://github.com/Flip451/SoTOHE-core/releases/tag/sotp-v0.1.2

## Compatibility checks (consumer)

Locally against this worktree after installing the `sotp-v0.1.2` binary:

- `bin/sotp task-contract coverage` → exit 0
- `bin/sotp task-contract check` → exit 0
- `bin/sotp test-obligation check` → exit 0 (`resolved_edges=3`)
- `cargo make ci-track` → exit 0

No other toolchain pins (Rust / cargo-make / cargo-deny / nextest) change in
this bump. CI continues to install via `cargo make install-sotp` from
`.harness/config/sotp-version.json`.

## Actions Taken

- [x] Research recorded under `knowledge/research/`
- [x] `.harness/config/sotp-version.json` tag set to `sotp-v0.1.2`
- [x] Local smoke of task-contract + test-obligation + `ci-track`
- [ ] Remote CI green on PR #2 (pending push)

## Recommendation

Pin `sotp-v0.1.2`. Do not stay on `sotp-v0.1.0` after an architecture-customizer
layer rename; do not stop at `sotp-v0.1.1` if the commit gate runs
`bin/sotp test-obligation check`.
