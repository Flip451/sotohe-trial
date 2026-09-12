# Web (Thin-Bin) Layer Review: Severity Policy

The reviewer's role is **thin-bin boundary review** of `apps/web/`
(`src/main.rs` and `src/commands/*.rs`). The bin layer obtains a wired HTTP server from
`web_composition`, maps its lifecycle result into the interface-adapter
`CommandOutcome`, and emits that outcome (stdout/stderr + `ExitCode`). It must not contain
business logic or import from `use_cases`/`entities` directly. Cite `architecture-rules.json`
when reporting a dependency-direction violation.

## Priority categories

Violations of the role statement above are always reportable. The following priority categories focus the review and guide severity assessment; they are not an exhaustive list of reportable design deviations. The exclusions in **What NOT to report** still apply:

- **external-boundary assumptions**: What external boundaries does the diff touch (OS, process, encoding, concurrency, resource limits, time, and other versions of its own artifacts)? Enumerate operations directly reached from the changed behavior. If a depended-on assumption is in neither the spec nor the environment-assumption declaration the project owns (the convention listed under `Current Files` in `knowledge/conventions/README.md` whose purpose is declaring environment assumptions; resolve it through that index, not a fixed filename), report it as `未宣言の前提への依存`; treat unresolvable indirect boundaries the same way rather than searching exhaustively.
- **business logic in bin**: a calculation, branching decision, entities/use_cases
  transformation, or multi-step orchestration in `src/main.rs`. The binary must
  stay a thin obtain-server-and-present wrapper. Computations that belong in
  `use_cases` or `entities` must not be inlined here. Name the computation and
  the layer that owns it.
- **direct I/O in bin**: production code in `apps/web/` that writes to files /
  the network for application purposes (e.g., telemetry persistence via
  `std::fs` / `serde_json` / `chrono::Utc::now()` inlined in `main.rs`).
  Permitted: `println!` / `eprintln!` in a tiny presenter that maps
  `CommandOutcome { message, success }` (or the server lifecycle outcome) to
  stdout/stderr and an exit code. Application-level I/O must not bypass
  `web_composition` / `interface_adapters`.
- **use_cases/entities import in production**: non-test code in `apps/web/` that
  imports from `use_cases::*` or `entities::*` directly (bypassing `web_composition`
  or `interface_adapters` re-exports). The bin's only knowledge of the application must
  come through `web_composition::*` (wiring) and `interface_adapters::*` (typed
  outcomes / controllers). Test-only imports (`#[cfg(test)]`) are excluded. Cite
  `architecture-rules.json` (`web` may depend only on `web_composition` and
  `interface_adapters`).
- **composition coordinate in bin violated**: the binary constructs adapters or
  invokes a use-case interactor directly (bypassing `web_composition::*CompositionRoot`).
  The bin obtains the assembled server and only presents its outcome.
- **ExitCode mapping inconsistency**: a command that on the unhappy path prints
  an error but returns `ExitCode::SUCCESS` (or vice-versa). Scripted callers
  will misinterpret the result.
- **inconsistent output for `--json` / `--quiet` flag**: a command that ignores
  its own output-mode flag, or emits diagnostics to stdout instead of stderr
  (breaks piped consumption).

## What NOT to report

- Subcommand naming (`--scope` vs `--group`) when the existing name is documented
  in `.claude/commands/` or `README.md`
- `clap` derive vs builder — both are valid project styles
- Output color / formatting choices that do not affect correctness or flag
  consistency
- "You could use `?` here" suggestions when the surrounding code uses an explicit
  `match` for context attachment
- Restructuring the `commands/` directory layout
- Help-text wording suggestions
- `unwrap()` / `expect()` inside `#[cfg(test)]` blocks
