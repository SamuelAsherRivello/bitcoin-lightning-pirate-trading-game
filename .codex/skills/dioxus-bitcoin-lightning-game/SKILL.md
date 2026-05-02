---
name: dioxus-bitcoin-lightning-game
description: Work on the Dioxus Bitcoin Lightning Game Rust workspace. Use when Codex is asked to modify, debug, verify, or explain this repository, especially Dioxus 0.7 UI, routing, assets, localization, browser localStorage snapshots, native SQLite template data, or project scripts.
---

# Dioxus Bitcoin Lightning Game

Use this skill for repository-specific execution context. Follow `AGENTS.md` and `.codex/rules/dioxus-0.7-workflow.md` first for Dioxus 0.7 rules.

## Start Here

1. Confirm the current directory is `D:\Documents\Projects\VC\Rust\dioxus-bitcoin-lightning-game`.
2. Read `.codex/rules/dioxus-0.7-workflow.md` for Dioxus implementation work.
3. Read the files directly involved in the request before editing.
4. Keep web and desktop support intact unless the request is explicitly platform-specific.
5. Prefer the existing `packages/ui`, `packages/web`, and `packages/desktop` boundaries.
6. Use the project scripts before inventing new commands.

## Workspace Map

| Path | Use |
| ---- | --- |
| `packages/ui/src/client/app.rs` | Shared app shell entry component. |
| `packages/ui/src/client/components` | Shared Dioxus components. |
| `packages/ui/src/client/pages` | Routed template pages. |
| `packages/ui/src/client/services` | Client template data, storage, localization, and database services. |
| `packages/ui/assets` | Shared CSS and localization/flag assets. |
| `packages/web/src/main.rs` | Web entrypoint. |
| `packages/desktop/src/main.rs` | Desktop entrypoint. |
| `Documentation/DioxusFeatureMatrix.md` | Dioxus feature matrix that should stay current as development continues. |
| `Scripts` | Windows PowerShell setup and run workflows. |

## Dioxus 0.7 Constraints

- Use Dioxus `0.7` patterns from the checked-in workspace dependency.
- Do not use `cx`, `Scope`, or `use_state`.
- Use `#[component] fn Name(...) -> Element`.
- Use `use_signal`, `use_memo`, `use_resource`, and signal `.read()`, `.write()`, `.with_mut()`, or call syntax.
- Use `Router::<Route> {}` and Dioxus router links for routing.
- Use `asset!("/path/from/project/root")` for local assets.
- Keep props owned, `Clone`, and `PartialEq`.

## Cache And Loading Behavior

- Preserve visible loading affordances during cache reads, database creation, and refreshes.
- Browser builds use localStorage snapshots instead of browser SQLite.
- Non-wasm builds use native SQLite under local `data/`.
- Put first-time native database/schema/seed setup in `create_database_if_missing()` and keep normal reads non-destructive.
- Treat stale dev servers as a common source of false browser results.

## Verification

Use the narrowest check that proves the change:

```powershell
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
.\Scripts\Common\RunWeb.ps1
.\Scripts\Common\RunDesktop.ps1
.\Scripts\Other\RunTests.ps1
```

For browser UI, routing, asset, or cache changes, serve the web app and inspect the actual page when practical.
