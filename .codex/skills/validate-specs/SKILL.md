---
name: validate-specs
description: Validate Spec Kit specs against the current codebase truth for a target feature, active feature, or all specs. Use when the user asks to validate specs, compare specs to implementation, audit spec/code drift, verify checked-off tasks, or decide whether code, specs, or both should be updated.
---

# Validate Specs

Use this skill to perform a read-only alignment pass between feature specs and the actual repository implementation. The goal is to identify whether the specs and code are 1:1 aligned, then ask for the remediation direction before editing anything.

## Target Resolution

Resolve the target before reading deeply:

1. Use a user-provided feature name, spec directory, route, package, module, service, page, or file path when present.
2. If no target is provided, inspect all directories under `specs/`.
3. If `.specify/feature.json` exists, read `feature_directory` as the active-feature hint. Use it as the target only when the user asks for the active/current feature or when it is the only spec.
4. If a named target matches both specs and code, validate both surfaces for that same target.
5. State the resolved target and why it was selected.

## Workflow

1. Read the spec truth.
   - Load the relevant `spec.md`, `plan.md`, `tasks.md`, checklists, and related files under the target `specs/<feature>/` directory when present.
   - Load `.specify/memory/constitution.md` when it exists.
   - Extract concrete requirements, routes, UI states, data behavior, cache behavior, platform support, acceptance criteria, task status, and documentation promises.
   - Do not rely on summaries alone when exact spec wording affects the decision.
   - Treat missing optional artifacts as facts, not errors. For example, a baseline spec may have `spec.md` and checklists but no `plan.md` or `tasks.md`.

2. Read the codebase truth.
   - Inspect the implementation files that actually own the target behavior.
   - Use direct evidence from code, tests, assets, routes, services, scripts, and docs. Prefer `rg`/file reads over assumptions.
   - Keep the pass read-only unless the user has already chosen a remediation direction.

3. Compare specs to code 1:1.
   - Mark each spec claim as `Aligned`, `Spec-only`, `Code-only`, or `Partial`.
   - Treat implemented behavior that is missing from specs as drift, not automatically as a success.
   - Treat checked-off tasks whose behavior is absent or different in code as drift.
   - Treat code that intentionally diverges from stale specs as drift until the specs are updated.
   - Include file paths and line numbers when practical.

4. Provide the analysis.
   - Default to chat for concise reports.
   - Write a Markdown report under `.codex/tmp/` only if the analysis is too large for a clear chat response or the user asks for a file.

5. Ask for remediation when not aligned.
   - If specs and code are not 1:1 aligned, stop before editing and ask:
     1. Update code to match specs (default)
     2. Update specs to match code
     3. Update both
   - Recommend the default only when the specs appear current and unambiguous.
   - Recommend updating specs or both when code clearly reflects later accepted behavior or when specs are incomplete.

## Dioxus Bitcoin Lightning Game Evidence Map

For this repo, validate common spec claims against these implementation owners:

| Spec Claim | Codebase Truth To Inspect |
| ---------- | ------------------------- |
| Routes and default page | `packages/ui/src/client/mod.rs`, `packages/ui/src/client/app.rs`, `packages/ui/src/client/components/page_header.rs` |
| Page structure and copy | `packages/ui/src/client/pages/page01.rs`, `page02.rs`, `page03.rs`, `template_page.rs`, localization bundles under `packages/ui/assets` |
| Template data text/source | `packages/ui/src/client/pages/page01.rs`, `packages/ui/src/client/models.rs`, `packages/ui/src/client/services/template_data_service.rs` |
| Browser localStorage snapshots/preferences | `packages/ui/src/client/services/storage_service.rs` and wasm-gated paths |
| Native SQLite setup/reads | `packages/ui/src/client/services/database_service.rs` and its tests |
| Top bar controls/toasts | `packages/ui/src/client/components/page_header.rs`, `developer_tools.rs`, `toast.rs` |
| Web and desktop entrypoints | `packages/web/src/main.rs`, `packages/desktop/src/main.rs` |
| Project docs promises | `Documentation/DioxusFeatureMatrix.md`, `README.md`, `AGENTS.md`, `.codex/rules/*` |
| Tests and task completion | `packages/ui/tests`, inline Rust tests, `tasks.md` checkbox state when present |

## Output Standard

Use this report shape:

```markdown
**Target**
<resolved spec/code target and selection reason>

**Verdict**
Aligned | Not aligned

**Evidence Checked**
- <spec artifact> -> <code/doc/test paths checked>

**Alignment Matrix**
| Spec Item | Status | Codebase Evidence | Notes |
| --------- | ------ | ----------------- | ----- |

**Drift Findings**
- <only include when status is Spec-only, Code-only, or Partial>

**Recommended Direction**
<update code, update specs, or update both, with one-sentence rationale>
```

Keep the report concrete and evidence-backed:

- Cite local files with paths and line numbers when practical.
- Separate spec gaps from code gaps.
- Do not propose broad rewrites when a small spec or code correction would restore alignment.
- If no drift is found, say the target is aligned and list the evidence checked.
- If verification was limited, state exactly what was not checked.
- Keep chat reports concise. Prefer a file under `.codex/tmp/validate-specs-<target>.md` for large all-spec audits.

## Remediation Rules

After the user chooses a direction, use the narrowest patch that restores alignment. For Dioxus behavior changes, also follow `.codex/rules/dioxus-0.7-workflow.md`, and update `Documentation/DioxusFeatureMatrix.md` when feature usage, routes, cache behavior, platform support, or future work changes.
