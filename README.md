# Local Tic-Tac-Toe (NestJS + Rust TUI)

Learning project with:
- Backend: NestJS REST API (`backend`)
- Frontend: Rust TUI with `ratatui` (`frontend-tui`)

## Features
- Solo mode vs simple built-in AI
- PvP mode with lobby
- Create PvP game with name and optional password
- Join open PvP game and play turn-by-turn
- Local only (no hosting, no paid libraries)

## Project structure
- `backend`: in-memory game sessions and REST endpoints
- `frontend-tui`: terminal UI, polling backend for live PvP updates

## Run backend
```bash
cd backend
npm install
npm run start:dev
```
Backend runs on `http://localhost:3000`.

## Run frontend TUI
In another terminal:
```bash
cd frontend-tui
cargo run
```

## Controls (TUI)
- Home: `Up/Down`, `Enter`
- Global: `q` to quit
- Game board: `Arrows` or `1..9`, `Enter/Space` to place move
- PvP lobby: `c` create, `j` join selected, `r` refresh, `b` back

## Backend API (used by TUI)
- `POST /games/solo`
- `POST /games/pvp`
- `GET /games/pvp/open`
- `POST /games/pvp/:gameId/join`
- `GET /games/:gameId`
- `POST /games/:gameId/move`

### API details

Base URL: `http://localhost:3000`

All request bodies are validated with NestJS `ValidationPipe`:
- unknown fields are rejected
- invalid DTO shapes return `400 Bad Request`

#### `POST /games/solo`
Create a new solo game (`X` = human host, `O` = AI).

Request body:
```json
{
  "playerId": "b4f0a8a3-0f56-4f58-9a2a-21f5e4b4db7a",
  "clientName": "My TUI Client"
}
```

#### `POST /games/pvp`
Create a new PvP lobby.

Request body:
```json
{
  "playerId": "b4f0a8a3-0f56-4f58-9a2a-21f5e4b4db7a",
  "name": "Friday Duel",
  "password": "optional123"
}
```

Rules:
- `name`: 3..40 chars
- `password`: optional, 3..32 chars

#### `GET /games/pvp/open`
List open PvP games (`status = WAITING_FOR_PLAYER`).

#### `POST /games/pvp/:gameId/join`
Join an open PvP game as guest player (`O`).

Request body:
```json
{
  "playerId": "65f9dfdb-c5fd-4c71-b18f-b84e3ad3a06a",
  "password": "optional123"
}
```

#### `GET /games/:gameId`
Fetch current game state by id.

#### `POST /games/:gameId/move`
Play one move.

Request body:
```json
{
  "playerId": "b4f0a8a3-0f56-4f58-9a2a-21f5e4b4db7a",
  "index": 4
}
```

Rules:
- `index` is `0..8`
- not your turn -> `401 Unauthorized`
- occupied cell / inactive game -> `400 Bad Request`

### GameState response shape

All successful endpoints return a game object (or array of game objects for `GET /games/pvp/open`):

```json
{
  "id": "2d5f4a45-cf8f-46f8-9f07-31a4bc0dc8f5",
  "mode": "PVP",
  "name": "Friday Duel",
  "hostPlayerId": "b4f0a8a3-0f56-4f58-9a2a-21f5e4b4db7a",
  "guestPlayerId": "65f9dfdb-c5fd-4c71-b18f-b84e3ad3a06a",
  "board": ["X", null, "O", null, null, null, null, null, null],
  "currentTurn": "X",
  "status": "IN_PROGRESS",
  "winner": null,
  "createdAt": "2026-02-27T10:00:00.000Z",
  "updatedAt": "2026-02-27T10:00:02.000Z",
  "hasPassword": true
}
```

Notes:
- `password` is never returned from the API.
- In solo mode, AI may play immediately after your move before the response is returned.

### Local Swagger / OpenAPI

A local spec is provided at `backend/openapi.yaml`.

Option 1 (Docker, Swagger UI):
```bash
docker run --rm -p 8080:8080 \
  -e SWAGGER_JSON=/spec/openapi.yaml \
  -v "$(pwd)/backend/openapi.yaml:/spec/openapi.yaml" \
  swaggerapi/swagger-ui
```
Open `http://localhost:8080`.

Option 2 (online editor):
- Open https://editor.swagger.io and paste `backend/openapi.yaml` content.

## Notes
- Data is stored in memory, so restarting backend resets all games.
- AI is intentionally simple and readable: win, block, center, corners, fallback.
