# Bitcoin Lightning Pirate Trading Game Constitution

## Core Principles

### I. Dioxus 0.7 Is The UI Contract
All UI work MUST use Dioxus 0.7 patterns. Components use `#[component] fn Name(...) -> Element`, signals use `use_signal`, and routes use the `Routable` enum with `Router::<Route> {}`. Removed APIs such as `cx`, `Scope`, and `use_state` are not acceptable in this project.

### II. Shared UI, Platform Entry Points
Product behavior belongs in `packages/ui` unless it is truly platform-specific. `packages/web` and `packages/desktop` remain thin launch and asset surfaces. Changes MUST preserve both browser and desktop support unless a spec explicitly narrows the platform scope.

### III. Visible Loading And Lab Feedback
Template data loading, cache reads, cache writes, refreshes, database creation, Lightning lab setup tests, route changes, invoice creation, payment attempts, and block waits MUST remain visible to users through the app's loading and toast-style status feedback. Browser builds MUST use localStorage snapshots rather than browser SQLite or OPFS worker startup.

### IV. Local Preferences Stay Local
Theme, language, non-sensitive Lightning lab setup preferences, and browser demo lab snapshots remain local user preferences. UI preferences MUST be persisted through the storage service rather than the SQLite database. Sensitive Lightning credentials belong in a server-side lab profile by default, not browser local storage.

### V. Database Creation Is Explicit
First-time native database/schema/seed setup MUST live in `create_database_if_missing()` in the database service. Normal read paths MUST NOT recreate, clear, or reseed an existing database.

### VI. Verify Real Behavior
Browser-visible changes SHOULD be checked against the served web app when practical. Use repository scripts for repeat workflows, especially `.\Scripts\Common\RunWeb.ps1`, `.\Scripts\Common\RunDesktop.ps1`, and `.\Scripts\Other\RunTests.ps1`. Compile success alone is not enough for browser runtime or visible UI issues.

### VII. Rust And Dioxus Coding Standards
All code MUST follow idiomatic Rust formatting, naming, ownership, and error-handling practices. Use `rustfmt` default formatting, Rust naming conventions (`snake_case` for functions/modules, `UpperCamelCase` for types/components, and `SCREAMING_SNAKE_CASE` for constants), and meaningful `Result` errors for expected failures instead of panics or `unwrap()` in runtime paths.

Dioxus code MUST follow Dioxus 0.7 component and reactivity standards. Components rendered from RSX use `#[component] fn Name(...) -> Element`; props are owned and implement `Clone` and `PartialEq`; reusable props may use `ReadOnlySignal<T>` when reactive tracking is required. State uses `use_signal`, `use_memo`, `use_resource`, context signals, and signal read/write APIs. Removed APIs such as `cx`, `Scope`, and `use_state` are forbidden.

RSX SHOULD stay declarative and readable: use component syntax inside RSX, direct loops for simple repeated UI, route-aware navigation under `Router::<Route> {}`, and `asset!("/assets/...")` plus document stylesheet components for bundled assets. Keep shared UI behavior in `packages/ui`; keep platform entrypoints thin.

## Project Constraints

- Keep the workspace split between `packages/ui`, `packages/web`, and `packages/desktop`.
- Keep static app chrome localizable through the Fluent locale assets in `packages/ui/assets/i18n/`.
- Preserve the top navigation order: `Set Up`, `Play Game`, `Debug Network`.
- Keep direct LND access behind `packages/lightning-service` and app-level server-operation contracts. Browser UI must not directly handle production or mainnet node credentials.
- Keep `Documentation/DioxusFeatureMatrix.md` updated as development continues.
- Keep `Documentation/Images/Screenshot01.png` and `Documentation/Images/Infographic01.png` as replaceable image slots.
- Do not introduce broad app redesigns or unrelated refactors while implementing a feature spec.

## Development Workflow

1. Inspect current files before editing and keep changes scoped to the requested behavior.
2. Prefer project scripts over ad hoc commands for setup, web serving, desktop serving, and tests.
3. For Dioxus or dependency guidance, use current Dioxus 0.7 documentation and the project-local Codex rules before changing code.
4. If a port or build artifact is stale or locked, diagnose the actual process or path instead of assuming a clean environment.
5. Treat `target/`, `node_modules/`, runtime data, and browser-test output as generated artifacts unless a spec explicitly says otherwise.

## Governance

This constitution applies to all future Spec Kit specifications, plans, and task lists for this repository. Specs may add narrower acceptance criteria, but they must not contradict these principles without explicitly updating this constitution and documenting the reason.

**Version**: 1.2.0 | **Ratified**: 2026-04-30 | **Last Amended**: 2026-05-02
