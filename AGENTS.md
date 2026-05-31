# AGENTS.md

This file is the shared operating guide for AI agents working in this repository. Keep it small, concrete, and current. Prefer real paths, command formats, and examples over broad advice.

## Mission

Build and maintain the Dioxus Bitcoin Lightning Game demo as a Rust 2021 workspace with shared Dioxus 0.7 UI, web and desktop entrypoints, local/regtest Lightning lab behavior, and durable documentation.

This repo is not a place for speculative rewrites. Inspect the relevant files first, make focused diffs, preserve web and desktop support, and verify with the narrowest command that proves the change.

## First Context To Load

- `AGENTS.md`: this file.
- `.codex/rules/dioxus-0.7-workflow.md`: Dioxus implementation workflow.
- `specs/010-nostr-profile/plan.md`: current Spec Kit implementation plan and feature constraints.
- The files directly involved in the request.

## Project Map

- `packages/ui/src/client/mod.rs`: route enum, app-level context providers, shared app wiring.
- `packages/ui/src/client/app.rs`: shared app shell entry component.
- `packages/ui/src/client/pages`: routed pages.
- `packages/ui/src/client/components`: shared Dioxus components.
- `packages/ui/src/client/components/setup`: setup wizard components.
- `packages/ui/src/client/components/game`: gameplay components.
- `packages/ui/src/client/components/network`: dashboard/network components.
- `packages/ui/src/client/services`: Dioxus-facing services, browser localStorage snapshots, native SQLite template data adapters, server-function wrappers.
- `packages/ui/src/client/models.rs`: UI-facing model exports and app models.
- `packages/ui/assets`: shared CSS and localization/flag assets.
- `packages/web/src/main.rs`: web entrypoint.
- `packages/desktop/src/main.rs`: desktop entrypoint.
- `packages/lightning-service`: reusable Lightning lab, TRA, auth, and policy service boundary.
- `Documentation/DioxusFeatureMatrix.md`: current Dioxus feature/platform matrix; update it when feature usage, routes, cache behavior, platform support, or suggested future work changes.
- `Scripts`: Windows PowerShell setup, run, and test workflows.

## Preferred Commands

Use repository scripts for repeat workflows:

```powershell
.\Scripts\Common\InstallDependencies.ps1
.\Scripts\Common\RunWeb.ps1
.\Scripts\Common\StopWeb.ps1
.\Scripts\Common\RunDesktop.ps1
.\Scripts\Other\RunTests.ps1
```

Use focused checks first:

```powershell
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
cargo test -p lightning-service
```

For browser-visible changes, serve the real web app and verify the page when practical. Use a concrete local IPv4 address instead of `0.0.0.0` for Dioxus fullstack web testing on Windows; wildcard binding can make backend readiness fail with `os error 10049`.

If a dev server is already running on the target port, stop that server and restart this project so the browser points at the latest build.

## Dioxus 0.7 Rules

- Treat the Dioxus 0.7 docs as authoritative: https://dioxuslabs.com/learn/0.7/
- Dioxus 0.7 changed every API. Do not use older APIs such as `cx`, `Scope`, or `use_state`.
- Components rendered from RSX use `#[component] fn Name(...) -> Element`.
- Component names must start with a capital letter or contain an underscore.
- Props must be owned, `Clone`, and `PartialEq`. Prefer `String`, `Vec<T>`, `Option<T>`, `ReadOnlySignal<T>`, or local model types over `&str` and borrowed slices.
- Use `use_signal`, `use_memo`, `use_resource`, context signals, and signal `.read()`, `.write()`, `.with_mut()`, `.set()`, `.peek()`, or call syntax as appropriate.
- Use `.peek()` for cache writes, toast deduplication, and background persistence logic where a reactive subscription would create a feedback loop.
- Use direct `for` loops and `if` blocks in `rsx!` when they are clearer than iterator chains.
- Use `Router::<Route> {}`, `#[derive(Routable, Clone, PartialEq)]`, `#[layout(...)]`, and `#[route("/path")]` for routing.
- Use `asset!("/assets/...")` for local assets relative to the package root.
- Inject styles with the existing Dioxus document pattern.

## API Docs

- Dioxus API docs: https://docs.rs/dioxus/latest/dioxus/
- Rust API design reference: https://github.com/apollographql/rust-best-practices

## Product And Route Rules

- Preserve the workspace split: shared UI and business logic in `packages/ui`, web entrypoint in `packages/web`, desktop entrypoint in `packages/desktop`.
- Keep both web and desktop support intact unless a request is explicitly platform-specific.
- The primary product routes, from left to right, are `Home`, `Set Up`, `Play Game`, and `Network Dashboard`.
- `Home` owns why the demo exists and the FAQ/concepts content.
- Keep route locking connection-based through `SetupProfile::is_connected()` unless a request explicitly changes that contract.
- Do not block Play Game navigation just because LNAuth login is missing; Play Game owns the login QR modal on first arrival.

## Setup Flow Rules

Polar setup steps have a required visual order, and execution/progression logic must match the same order exactly:

1. `Bridge URL`
2. `Server Name`
3. `Create Nodes`
4. `Game Treasury (Sats)`
5. `Game Treasury (TRAs)`
6. `User Nodes (Sats)`
7. `User Nodes (TRAs)`
8. `Block Height`
9. `Unlock Routes`

`Create Nodes` finds or creates every Polar node needed by later steps: the Bitcoin backend, Game Treasury LND node, `GAME_TAPROOT` Taproot Assets node, two NPC LND nodes, and one player LND node. It requests creation first, restarts the Polar network after node topology changes such as creates, renames, or cleanup removals, then checks all node start/status readiness. If repeated readiness polls show nodes stuck in the same non-started state, restart the Polar network once and re-check.

Required Polar node names are exact and case-sensitive in app requests:

- `BITCOIN_TESTNET`: the Polar Bitcoin backend / bitcoind node.
- `GAME_LND`: the Game Treasury LND node.
- `GAME_TAPROOT`: the Taproot Assets node attached to `GAME_LND` and `BITCOIN_TESTNET`.
- `Jack`: the player LND node.
- `Bob`: NPC LND node 1.
- `Carol`: NPC LND node 2.

`Server Name` only finds, creates, and starts the named Polar server. It must not delete, rename, or add non-server nodes.

Polar node mutation has lifecycle side effects. The live Polar MCP tool schema documents that `rename_node` temporarily stops a running network, `set_lightning_backend` restarts the affected Lightning node when the network is running, and `remove_node` stops the removed node. Treat those operations as disruptive: setup steps must inspect first, preserve already-usable manual Polar state, and avoid rename/remove/backend rewiring unless a required node or backend is genuinely missing and the user is in the owning setup step.

`Create Nodes` owns topology reconciliation:

1. Get the full Polar node list for the selected server.
2. If every required node already exists by exact name and all existing nodes are started/running, mark the step ready and report extra nodes without deleting, renaming, stopping, or restarting anything.
3. Create any required node from the exact required list that does not exist yet. Do not rename existing nodes into the required names; node rename can stop a running network.
4. After the exact required list exists, request start only for required nodes that are not already started/running.
5. Poll required nodes every 3 seconds until they are started. If there is no progress after 3 polls, stop/start the network once as the Polar stability workaround, then continue polling.

Exception: if Polar already has a Taproot Assets node under a generated or legacy name such as `GAME_LND-tap`, `GAME_TREASURY-tap`, or `tapd`, treat the proper Taproot node type as usable state. Do not delete or rename it just because the name is wrong. Continue with the existing Taproot node name. Only when no Taproot node exists at all should setup create one with the preferred name `GAME_TAPROOT`.

`User Nodes (Sats)` and `User Nodes (TRAs)` rebalance existing or newly created networks by transferring value to or from Game Treasury until the player/NPC user nodes match the requested demo balances and inventory. Game Treasury may retain extra sats or TRAs after rebalancing; do not require exact treasury depletion.

Keep the existing `Connection`/environment tabs unless the task explicitly asks to change them. `User Auth` is a user authorization selector, not a replacement for the Polar bridge flow.

## Cache, Storage, And Loading

- Preserve visible loading or toast-style status feedback during template data loading, cache reads, cache writes, database creation, setup testing, route opening, block waits, invoice creation, and payment attempts.
- Browser builds use localStorage snapshots for template data cache behavior.
- Non-wasm builds use native SQLite for template data.
- Keep template data cache behavior in `packages/ui/src/client/services`.
- Keep first-time native database/schema/seed setup in the clearly named `create_database_if_missing()` service method.
- Normal reads must not recreate, clear, or reseed an existing database.
- Do not introduce browser SQLite or OPFS worker startup for template data caching.
- Lightning lab behavior lives behind `packages/ui/src/client/services/lightning_server_functions.rs` and `packages/lightning-service`.
- Browser builds persist only non-sensitive setup preferences and demo lab snapshots.
- Never persist wallet secrets, macaroons, seed material, private keys, cookies, API tokens, or database credentials in browser storage, generated docs, logs, commits, screenshots, or PR text.

## Good Patterns To Copy

- Routes and context: `packages/ui/src/client/mod.rs`.
- Page layout composition: `packages/ui/src/client/components/page.rs`, `page_header.rs`, and `page_footer.rs`.
- Toast/status UX: `packages/ui/src/client/components/toast.rs`.
- Setup field help and wizard patterns: `packages/ui/src/client/pages/setup.rs` and `packages/ui/src/client/components/setup`.
- Gameplay state display and actions: `packages/ui/src/client/pages/play_game.rs` and `packages/ui/src/client/components/game`.
- Network/dashboard display: `packages/ui/src/client/pages/debug_network.rs` and `packages/ui/src/client/components/network`.
- Browser/native storage split: `packages/ui/src/client/services/storage_service.rs`, `template_data_service.rs`, and `database_service.rs`.
- Lightning service boundary: `packages/lightning-service/src`.

## Avoid

- Do not copy old Dioxus examples that use `cx`, `Scope`, borrowed props, or `use_state`.
- Do not put reusable business logic directly in page components when it belongs in `packages/lightning-service` or `packages/ui/src/client/services`.
- Do not add new heavy dependencies without clear need and user-visible benefit.
- Do not introduce platform-specific code into shared modules without `cfg` gates or an existing abstraction.
- Do not make broad repo-wide rewrites for a local behavior change.
- Do not remove visible loading, progress, toast, or error states around async work.

## Test-First Guidance

- For regressions, add or update a test that reproduces the bug before fixing it when practical.
- For new service behavior, prefer unit tests in the owning service crate.
- For Dioxus UI behavior, use focused Rust checks plus browser-visible verification when the change affects rendered routes, assets, cache reads/writes, or user flows.
- Broaden to `.\Scripts\Other\RunTests.ps1` when changes cross package boundaries or affect shared contracts.

## PR / Handoff Checklist

Before handing off code changes:

- Diff is small and focused on the requested behavior.
- Relevant focused checks were run, or any skipped check is explained.
- Browser-visible changes were verified in the served web app when practical.
- Web and desktop support remain intact.
- User-facing behavior changes are reflected in `Documentation/DioxusFeatureMatrix.md` when applicable.
- Excessive debug logs, temporary comments, and scratch artifacts are removed.
- Secrets are not printed, copied, persisted, or committed.

## When Stuck

Ask a concise clarifying question, propose a short plan, or document the blocker. Do not push large speculative changes, rewrite architecture, add dependencies, expose services publicly, or change storage/database behavior because of an assumption.

## Git Safety

- Never run destructive Git operations.
- User permission does not override this rule.
- Git use is limited to inspection, additive work, commits, branch creation/switching, and non-destructive merges.
- Do not delete repositories.
- Do not delete existing commits.
- Do not rewrite Git history.
- Do not run `git reset --hard`.
- Do not run `git clean`.
- Do not run `git rebase`.
- Do not run `git commit --amend`.
- Do not squash commits.
- Do not force-push.
- Do not run `git push --force` or `git push --force-with-lease`.
- Do not run `git checkout -- <path>`, `git restore <path>`, or similar commands that discard local file changes.
- `git checkout` is allowed only for creating or switching branches.
- Do not run `git pull --rebase`.
- Do not run `git branch -D` or delete local or remote branches.
- Do not delete tags.
- Do not overwrite tags.
- Allowed Git operations are `git status`, `git diff`, `git log`, `git show`, `git fetch`, `git branch` for creating/listing branches, `git checkout` for creating/switching branches, `git add`, `git commit`, `git merge`, and normal non-force `git push`.
- If a task appears to require destructive Git, stop and explain that the operation is not permitted.

## Secret And Credential Safety

- Never ask the user to paste passwords, private keys, seed phrases, wallet descriptors, API keys, cookies, session tokens, or database credentials into chat.
- Never print secrets from local files, remote files, environment variables, GitHub secrets, `.env` files, SSH configs, wallet files, or server configs.
- If a task needs a secret, instruct the user to enter it directly into the target app, terminal, GitHub secret field, password manager, or server file.
- Do not copy secrets into generated files, logs, commits, pull requests, screenshots, or Markdown notes.
- Do not commit `.env` files, SSH private keys, wallet files, database dumps, or backup archives.
- When handling public repositories, assume all committed workflow files, scripts, README text, and config examples are public.

## Workspace And Server Scope

- Stay inside this repository unless the user explicitly names another path or repository in the current request.
- Do not read, create, edit, move, rename, delete, or otherwise operate on files outside this project repository unless the user explicitly names another repository as part of the current request.
- Keep generated files, scratch files, downloaded assets, caches, and temporary outputs inside this repository.
- Do not delete folders outside this repository.
- Do not change global Codex config, SSH config, Git config, Windows services, or remote server firewall rules unless the user explicitly asks for that class of change.
- Do not expose local-only services, admin dashboards, Bitcoin RPC, Lightning RPC, databases, Docker sockets, or app internals to the public internet without explicit approval.
- Prefer localhost, SSH tunnels, VPN, or Tor hidden services for private admin access.
- Before opening public ports, state the port, service, purpose, and risk.

## Database Safety

- Never perform destructive database operations.
- User permission does not override this rule.
- Reads, inserts, and non-destructive updates are allowed when scoped to the task.
- Do not delete rows.
- Do not delete tables.
- Do not drop databases.
- Do not truncate tables.
- Do not run destructive schema migrations.
- Do not run SQL containing `DELETE`, `DROP`, `TRUNCATE`, destructive `ALTER`, or equivalent ORM/migration operations.
- If a task appears to require destructive database work, provide the exact command or SQL for the user to review and run manually.

<!-- SPECKIT START -->
For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan:
`specs/010-nostr-profile/plan.md`
<!-- SPECKIT END -->
