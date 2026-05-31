# Codex Project Boilerplate

Use this folder for Codex-specific project context that should travel with this repository. Spec Kit workflow skills live in `.agents/skills`; keep `.codex` focused on project-specific rules for the Bitcoin Lightning Pirate Trading Game.

## Project

`bitcoin-lightning-pirate-trading-game` is a Rust workspace for a Dioxus 0.7 template app with shared UI code and separate web and desktop entrypoints.

| Path | Purpose |
| ---- | ------- |
| `packages/ui` | Shared Dioxus code split between pages, components, models, client services, assets, and tests. |
| `packages/web` | Web app entrypoint and web assets. |
| `packages/desktop` | Desktop app entrypoint and desktop assets. |
| `Scripts` | Windows PowerShell workflows for setup, web run, desktop run, and tests. |
| `Documentation/Images` | README screenshots and infographic assets that can be replaced in-place. |

## Project Rules

Use `.codex/rules/dioxus-0.7-workflow.md` for Dioxus implementation, debugging, routing, state, asset, async loading, cache, or cross-platform work.

Use `.codex/rules/frontend-design.md` when changing user-visible UI, CSS, layout, component composition, route presentation, loading states, toasts, empty/error states, or visual assets.

Keep `Documentation/DioxusFeatureMatrix.md` updated when the template's Dioxus feature usage or suggested next work changes.

## Default Commands

Prefer the repository scripts when possible:

```powershell
.\Scripts\Common\InstallDependencies.ps1
.\Scripts\Common\RunWeb.ps1
.\Scripts\Common\StopWeb.ps1
.\Scripts\Common\RunDesktop.ps1
.\Scripts\Other\RunTests.ps1
```

For direct Dioxus web work, use:

```powershell
dx serve --platform web --addr 127.0.0.1 --port 8080
```

The web script stops an older `dx serve` process on the requested port before starting a new one. Use `.\Scripts\Common\StopWeb.ps1` to stop the Dioxus web server and generated game server without starting a replacement. If you run `dx serve` directly and port `8080` is already occupied, stop the old server and restart it so browser testing uses the latest build for this checkout.

## Runtime Notes

- The app supports web and desktop paths.
- Browser template data uses localStorage snapshots.
- Native template data uses SQLite under local `data/`.
- First-time database/schema/seed creation belongs in `create_database_if_missing()`.
- Browser behavior should be validated in a real served app when UI, routing, cache loading, or asset behavior changes.

## Skill Work

Use `.codex/skills/validate-specs/SKILL.md` to compare Spec Kit specs against the current codebase truth before choosing whether to update code, specs, or both.
