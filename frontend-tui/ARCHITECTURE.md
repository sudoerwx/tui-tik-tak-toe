# Frontend TUI Architecture

This document explains the Rust TUI app using TypeScript/React mental models.

## High-level map

- `src/main.rs`: app bootstrap and terminal lifecycle.
- `src/app.rs`: main app state + event loop + input handlers.
- `src/api.rs`: HTTP client/service layer (`reqwest`).
- `src/models.rs`: shared data types (DTOs, enums).
- `src/ui.rs`: pure rendering functions (`ratatui` widgets/layout).

Think of this as:

- `main.rs` -> Next.js `server.ts`/runtime bootstrap.
- `app.rs` -> root component + reducer/state machine.
- `api.rs` -> API service module.
- `models.rs` -> TS interfaces/types.
- `ui.rs` -> presentational components.

## Runtime flow

1. `main` enables raw terminal mode and alternate screen.
2. `App::run()` enters the loop.
3. Each tick:
   - Optional polling refresh from backend.
   - Draw current screen.
   - Read keyboard input and dispatch by screen.
4. On exit, terminal is restored.

## State model (`app.rs`)

- `App` struct is the single source of truth for UI/game state.
- `screen: Screen` is a finite-state machine:
  - `Home`, `SoloGame`, `PvpLobby`, `PvpCreate`, `PvpGame`, `GameOver`, `Info`.
- Handlers like `handle_home_key`, `handle_pvp_game_key` are equivalent to reducer/event handlers in React.

## API layer (`api.rs`)

- `ApiClient` wraps `reqwest::Client`.
- Each backend endpoint is a typed async method.
- `parse_json_response<T>()` centralizes:
  - HTTP status check.
  - JSON parsing.
  - consistent error messages.

This is similar to a typed `fetch` wrapper in TS.

## UI layer (`ui.rs`)

- Drawing functions are mostly pure: they receive data and render widgets.
- Layout is done with `Layout` + `Constraint` (terminal equivalent of CSS grid/flex sections).
- Keeping rendering outside `App` helps testability and readability.

## Rust concepts used (quick)

- `struct`: like a TS object shape with concrete fields.
- `enum`: tagged union for screen states.
- `Option<T>`: `T | null` equivalent.
- `Result<T, E>`: typed success/error return, similar to explicit error handling.
- `async/await`: same concept as TS, but with compile-time checked types and ownership rules.

## Why this structure is modern/maintainable

- Clear separation of concerns.
- Small modules with focused responsibilities.
- Strongly typed boundaries between state, API, and UI.
- Easier to extend (new screen/API call/widget without touching everything).

## Suggested next improvements

1. Add unit tests for pure helpers (`render_board_text`, cursor movement).
2. Introduce an app-level `Action` enum to unify input handling.
3. Add websocket support for PvP updates (replace polling loop).
