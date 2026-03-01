# Citordle

Citordle is a city-themed daily puzzle game split into a Rust backend and an Astro frontend.

Each day:
- Round 1 gives a Wordle-like word tied to the daily city.
- Round 2 asks a geography question with a nameless mini map.
- Round 3 is randomly chosen for the day: Duolingo-style prompt, drawing prompt, or trivia.
- A final success screen shows how many tries each round took.

## Project layout

- `backend/` - Rust API (Axum, modular design)
- `frontend/` - Astro app with React islands

## City data loading

- The backend loads city data from `backend/data/cities.json`.
- It also loads any extra `*.json` files directly under `backend/data/` (for example `world_cities_game_pack_60.json`).
- You can drop new city JSON files into `backend/data/cities/` to add or override cities without editing the base file.
- Each JSON file may contain either one city object or an array of city objects.
- If two files share the same `id`, the later-loaded file overrides the earlier one.

Daily selection is now cycle-based so each day gets a different city until the full city list is exhausted.

## Run locally

1. Start the backend:

```bash
cargo run --manifest-path backend/Cargo.toml
```

2. Install frontend dependencies with Bun:

```bash
bun install --cwd frontend
```

3. Start the frontend:

```bash
bun --cwd frontend dev
```

Backend defaults to `http://localhost:8080` and frontend to `http://localhost:4321`.

Set `PUBLIC_API_BASE` in the frontend environment if the API runs elsewhere.

For phone testing on your local network, run backend and frontend on your machine, then start Astro with host exposure:

```bash
bun --cwd frontend dev --host
```

By default the frontend now proxies `/api` and `/health` to `http://127.0.0.1:8080` in dev mode, so your phone can call the backend through the same exposed frontend URL.

## Daily progress token

- The backend issues a signed JWT session token for anonymous players.
- The frontend stores this token in local storage and sends it back on API requests.
- This keeps in-progress and completed state for the current daily puzzle across refreshes.

Optional backend env var:

```bash
JWT_SECRET=change-this-in-production
```
