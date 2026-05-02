---
name: list-skill-repetoir
description: "List every available Codex skill for this repository in a readable table with a 5-10 word summary and zero-padded repo-local usage count. Use when the user asks for the skill repertoire, wants to track which skills they have tried, asks for skill call counts, or wants to bump a skill's repo-local usage counter."
---

# List Skill Repetoir

## Purpose

Maintain a lightweight repo-local skill repertoire so the user can see every available skill, remember what each one does, and track which skills they have tried in this repository.

## Data Source

Use `.cache/list-skill-repetoir/skill-counts.tsv` as the source of truth. It has three tab-separated columns:

```text
skill	summary	call_count
```

Keep `call_count` as a three-digit, zero-padded integer string such as `000`, `001`, or `012`.

## Workflow

1. Read `.cache/list-skill-repetoir/skill-counts.tsv`.
2. If the user asked to bump a skill, run `.codex/skills/list-skill-repetoir/scripts/render_skill_repertoire.ps1 -Bump <skill-name>` from the repository root.
3. If the user only asked to list the repertoire, run `.codex/skills/list-skill-repetoir/scripts/render_skill_repertoire.ps1`.
4. Return the generated Markdown table directly.
5. If new skills are available in the session but missing from the TSV, add them with a 5-10 word summary and `000` before rendering.

## Output Format

Use a Markdown table with exactly three columns:

```text
| Skill | Summary | CallCount |
| --- | --- | --- |
| skill-name | Five to ten word summary | 000 |
```

Keep summaries short and concrete. Preserve the user's misspelled skill name `list-skill-repetoir` unless they explicitly ask to rename it.

## Counting Rules

- Increment a skill when the user explicitly invokes it with `$skill-name` or clearly asks Codex to use that skill.
- Increment only once per user request, even if the workflow reads the skill multiple times.
- Do not infer historical counts from memory unless the user explicitly asks for a backfill.
- When uncertain, leave the count unchanged and mention the uncertainty briefly.
