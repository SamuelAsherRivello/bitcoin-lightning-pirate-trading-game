---
name: codex-yolo
description: "Plan cautious use of the external codex-yolo tmux launcher for parallel Codex agents. Use when the user asks about codex-yolo, wants to install or run codex-yolo, or wants help designing isolated parallel agent tasks with explicit safety boundaries."
---

# Codex YOLO

## Purpose

Help the user reason about `codex-yolo` without silently installing or launching it. Treat it as a high-risk automation wrapper because the upstream README says it auto-approves Codex CLI permission prompts and warns against use on corporate hardware or sensitive networks.

## Workflow

1. Confirm the user wants to use `codex-yolo`, not ordinary Codex skills, worktrees, or GitHub workflows.
2. Inspect the current repo state and identify whether the requested tasks are independent enough for parallel agents.
3. Recommend isolation before any run: disposable machine or VM, no personal data, no saved credentials, no sensitive network access, and explicit branch/worktree boundaries.
4. Do not install, launch, or resume `codex-yolo` unless the user explicitly asks for that action after seeing the risk summary.
5. Prefer `--worktree --no-merge` for code tasks so each agent has an isolated branch and the user can inspect changes before integration.
6. Do not enable no-sandbox behavior or public network exposure unless the user explicitly requests it and the environment is disposable.

## Output

For planning requests, provide:

- A concise risk note.
- A proposed `codex-yolo` command only when the user asked for a runnable command.
- One task string per agent, scoped narrowly.
- A verification and review step before any merge or cleanup.
