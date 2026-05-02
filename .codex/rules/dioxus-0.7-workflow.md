# Dioxus 0.7 Workflow Rule

Use this rule for any Dioxus implementation, debugging, routing, state, asset, async loading, cache, or cross-platform work in this repository.

Primary docs: https://dioxuslabs.com/learn/0.7/

## First Pass

- Read `AGENTS.md`, this file, and the files directly involved in the requested behavior before editing.
- Keep shared UI and business logic in `packages/ui`; keep target entrypoints in `packages/web` and `packages/desktop`.
- Preserve both web and desktop unless the request is explicitly platform-specific.
- Prefer existing app patterns before adding new abstractions. This template uses context signals for theme, language, template data load requests, cached template data results, and toast state.
- Treat Dioxus 0.7 docs as authoritative. Do not use older Dioxus APIs such as `cx`, `Scope`, `use_state`, old router setup, or borrowed component props.

## Components And Props

- Components return `Element` and use `#[component]` when they are rendered from RSX.
- Component names must start with a capital letter or contain an underscore.
- Props must be owned, `Clone`, and `PartialEq`. Use `String`, `Vec<T>`, `Option<T>`, `ReadOnlySignal<T>`, or local model types instead of `&str` or borrowed slices.
- Use component syntax inside RSX, for example `TemplatePage { title, body_01, body_02, body_03, data_text }`; do not call component functions directly.
- For repeated UI, prefer direct `for` loops in RSX when the output is straightforward and easier to read.
- Keep target-specific platform code behind modules or `cfg` gates. Shared components should compile for both `wasm32-unknown-unknown` and desktop.

## State And Reactivity

- Use `use_signal` for local mutable state and context signals for shared app state.
- Read signal values with call syntax for cheap clones, `.read()` for borrowed reads, `.peek()` for non-subscribing reads, and write with `.set()`, `.write()`, or `.with_mut()`.
- Use `.peek()` inside cache-write, toast-deduplication, or background persistence logic when a reactive subscription would create a feedback loop.
- Use `use_memo` for derived values that should recalculate only when their dependencies change.
- Use `use_context_provider` in layout/root components and `use_context::<Signal<T>>()` in descendants for shared state. This repo provides app-level context in `packages/ui/src/client/mod.rs`.

## Async Loading

- Use `use_resource` for async data that should rerun when the signals it reads change.
- `Resource` reads return `None` while loading and `Some(value)` after completion. Preserve a visible loading or toast state for `None`.
- Use async event handlers or `spawn` for user-triggered work and timers.
- Avoid overlapping user-triggered loads unless the existing flow supports them. Gate refresh behavior with existing request signals or explicit loading state.
- Never block the first meaningful render on optional cache work when snapshot data can render first.

## Routing And Layout

- Keep routes in the single `Route` enum in `packages/ui/src/client/mod.rs`.
- The default route `/` renders `Page01`; `/page-02` renders `Page02`; `/page-03` renders `Page03`.
- Use `#[derive(Routable, Clone, PartialEq)]`, `#[route("/path")]`, `#[layout(AppLayout)]`, and `Router::<Route> {}`.
- Render router-aware navigation only under `Router::<Route> {}`.

## Assets And Styles

- Use `asset!("/assets/...")` for local files relative to the package root. Do not use absolute machine paths.
- Keep shared CSS and flag/localization assets under `packages/ui/assets` when used by shared UI.
- Inject styles with Dioxus document components already used in the repo.
- For browser-visible styling or asset changes, run the real web app and inspect it instead of trusting compile success alone.

## Template Data Cache Behavior

- Template data may come from browser snapshot data or native SQLite.
- Preserve status feedback for template data loading, cache reads, SQLite operations, errors, and database creation.
- Browser builds must not introduce SQLite or OPFS worker startup.
- Native first-time schema and seed setup belongs in `create_database_if_missing()` in `database_service.rs`.
- Normal database reads should not recreate, clear, or reseed an existing database.

## Documentation

- Keep `Documentation/DioxusFeatureMatrix.md` updated whenever current Dioxus feature usage or planned extension points change.

## Verification

Use the smallest check that proves the change, then broaden when the edit crosses packages or runtime surfaces:

```powershell
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
.\Scripts\Other\RunTests.ps1
.\Scripts\Common\RunWeb.ps1
.\Scripts\Common\RunDesktop.ps1
```

Use a concrete local IPv4 address instead of `0.0.0.0` for web testing on Windows. If port `8080` is already serving an older build, stop that server and restart this project before trusting browser results.
