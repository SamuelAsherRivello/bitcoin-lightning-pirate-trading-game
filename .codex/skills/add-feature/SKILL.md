---
name: add-feature
description: Add or iterate on a user-targeted feature in the existing Dioxus Bitcoin Lightning Game app, carrying it from feature target and spec refinement through scoped implementation, testing, user feedback loops, and spec/documentation updates. Use when the user asks to add, build, implement, wire, extend, polish, or continue a Dioxus app feature and provides or can provide a target behavior.
---

# Add Feature

Use this skill to carry a feature request from an initial target into working code in this repository. Keep the feature inside the existing app unless the user explicitly asks for a new app or separate prototype.

## Start

1. Confirm the target behavior in one sentence.
2. Read `AGENTS.md`, `.codex/rules/dioxus-0.7-workflow.md`, and any directly involved files before editing.
3. If the feature changes UI, layout, assets, route presentation, loading states, toasts, empty states, or error states, read `.codex/rules/frontend-design.md`.
4. Check `.specify/feature.json`; if it exists, treat its `feature_directory` as the active spec unless the user names another spec.
5. Ask only for information that blocks a safe implementation. Prefer a documented assumption when the choice is low-risk.

Useful intake questions, only when needed:

- What user action should start this feature?
- Which route, page, or component should own the feature?
- What visible result confirms it worked?
- Should the behavior persist locally, use template data, or stay UI-only?
- Does it need both web and desktop support?

## Spec Path

Choose the lightest spec workflow that protects the feature:

- Direct edit: Use for small or localized changes where the active spec already covers the feature shape.
- `speckit-specify`: Use when the user introduces a materially new feature or the current specs do not describe it.
- `speckit-clarify`: Use when the spec has unresolved choices that materially change scope, UX, persistence, or platform behavior.
- `speckit-plan` and `speckit-tasks`: Use when the feature crosses multiple modules, data services, routes, or platform targets.
- `speckit-analyze`: Use after generated plan/tasks when consistency across `spec.md`, `plan.md`, and `tasks.md` matters.
- `speckit-implement`: Use when a complete `tasks.md` exists and the user wants the planned tasks executed.

For direct spec edits, preserve the existing spec section order and keep requirements testable. Do not add implementation details to user-facing requirements unless the current spec already uses that level of detail for project constraints.

## Implementation

- Keep shared app behavior in `packages/ui`.
- Touch `packages/web` or `packages/desktop` only for platform entrypoints, platform-specific assets, or platform-specific runtime behavior.
- Follow Dioxus 0.7 patterns: `#[component]`, `Element`, `use_signal`, `use_memo`, `use_resource`, `Router::<Route> {}`, owned props, and `asset!`.
- Do not use removed Dioxus APIs such as `cx`, `Scope`, or `use_state`.
- Preserve browser localStorage snapshots and native SQLite boundaries for template data.
- Keep first-time native database/schema/seed setup in `create_database_if_missing()`.
- Preserve visible loading or toast-style feedback for data loading, cache reads/writes, errors, and database creation.
- Keep the template reusable: avoid product-specific decoration unless the requested feature is intentionally product-specific.

## Testing

Add tests when the feature changes behavior that can regress without visual inspection:

- Services, data loading, persistence, localization, routing, or pure logic usually need focused tests.
- UI-only layout and copy changes usually need compile plus real browser verification instead of brittle tests.
- Cross-platform behavior usually needs at least one wasm check and one desktop check.
- When using `tasks.md`, mark completed tasks with `[X]` as each task is finished.

Prefer the narrowest check that proves the change first:

```powershell
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
.\Scripts\Other\RunTests.ps1
```

For browser-visible changes, run the real app through `.\Scripts\Common\RunWeb.ps1` and inspect it when practical. If the target port is already serving an old build, restart the server before trusting the browser result.

## Iteration Loop

After implementation, summarize what changed, what was verified, and any remaining risk. If the user says the feature is incomplete or wrong:

1. Treat their feedback as the next feature target.
2. Re-check the relevant code/spec area instead of assuming the earlier implementation is correct.
3. Patch the smallest coherent behavior change.
4. Re-run the checks that prove the updated behavior.
5. Update specs/docs when the accepted behavior changes.

Continue the loop until the user confirms the behavior is complete or a real blocker remains.

## Documentation

- Update the active `specs/<feature>/spec.md`, `plan.md`, or `tasks.md` when implemented behavior changes the agreed feature contract.
- Update `Documentation/DioxusFeatureMatrix.md` whenever Dioxus feature usage, routes, cache behavior, platform support, or suggested future work changes.
- Update README or package README files only when the feature changes documented usage, screenshots, setup, or user-facing project capability.

## Done

Finish with working code, appropriate tests or browser verification, and updated specs/docs when needed. If verification cannot be completed, state the exact command or runtime check that was not run and why.
