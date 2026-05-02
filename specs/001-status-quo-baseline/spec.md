# Feature Specification: Dioxus Bitcoin Lightning Game Baseline

**Feature Branch**: `001-status-quo-baseline`  
**Created**: 2026-04-30  
**Status**: Baseline  
**Input**: Template baseline after removing copied app-specific behavior.

## User Stories & Testing

### User Story 1 - Navigate Template Pages (Priority: P1)

A visitor can open the app and move between `Page01`, `Page02`, and `Page03` from the shared top navigation.

**Independent Test**: Open the app and verify `/` shows `Page01`, `/page-02` shows `Page02`, and `/page-03` shows `Page03`.

### User Story 2 - See Template Data Boilerplate (Priority: P1)

A visitor sees a small local data example on `Page01` showing `DB data is hello`.

**Independent Test**: Open `Page01` and verify the title appears first, followed by the DB data line and generic body content.

### User Story 3 - Use Local UI Preferences (Priority: P2)

A visitor can toggle between two themes and change the language from the top bar. Preferences persist locally without using the SQLite database.

**Independent Test**: Change theme and language, reload the app, and verify the selected preferences remain active.

## Requirements

- **FR-001**: The system MUST provide exactly three top-level routes: `Page01`, `Page02`, and `Page03`.
- **FR-002**: The `/` route MUST render `Page01` by default.
- **FR-003**: Each page MUST share the same structure: title, then three generic lorem ipsum paragraphs.
- **FR-004**: Each page MUST start its lorem ipsum copy at a slightly different offset so route changes are obvious.
- **FR-005**: `Page01` MUST render the local data message as `DB data is hello` after the title.
- **FR-006**: Browser builds MUST use localStorage snapshots for template data and preferences.
- **FR-007**: Non-wasm builds MUST use native SQLite under local `data/` for template data.
- **FR-008**: First-time native database/schema/seed setup MUST live in `create_database_if_missing()`.
- **FR-009**: Normal database reads MUST NOT recreate, clear, or reseed an existing database.
- **FR-010**: The top bar MUST keep navigation, theme toggle, language dropdown, GitHub link, DB refresh action, and toast region.
- **FR-011**: The app MUST keep web and desktop entrypoints that launch the same shared UI.
- **FR-012**: `Documentation/DioxusFeatureMatrix.md` MUST describe the current Dioxus feature usage and stay current as development continues.
- **FR-013**: Future implementation work MUST follow the project coding standards for idiomatic Rust and Dioxus 0.7 APIs documented in the project constitution.

## Key Entities

- **Template Data**: A single local example row with id `1` and message `hello`.
- **Template Data Load Result**: The current template data, source, and load timestamp.
- **Template Data Source**: The origin of displayed data, represented internally as browser snapshot or database and surfaced to users as `Database`.

## Success Criteria

- **SC-001**: A visitor can open `/` and identify `Page01`.
- **SC-002**: A visitor can navigate to `Page02` and `Page03` without losing the app shell.
- **SC-003**: `Page01` displays `DB data is hello`.
- **SC-004**: Theme and language changes persist locally.
- **SC-005**: A future project can replace the template spec and content without removing the workspace structure.

## Assumptions

- This baseline is intentionally generic and ready to be repopulated with new project specs.
- The placeholder README images keep their current filenames and may be replaced in-place.
