# Repository Guidelines

## Project Structure & Module Organization
- Backend lives in `src/` (Axum server, auth, SSH handler, SeaORM entities). Entry is `src/main.rs`; shared configuration/state in `src/state.rs`.
- Frontend lives in `frontend/app/` (HTML shell, Lit/TS components, CSS). Built assets are emitted to `dist/` via Bun.
- Reusable HTML templates sit in `templates/`; static/public files are served from `public/` and `/dist`.
- Database migrations are versioned SQL files in `migrations/` and tracked by `scripts/migrate.ts`.
- Helper scripts are in `scripts/`; generated or runtime data (git repos, assets, SSH keys) are under `data/` (keep keys out of commits).

## Build, Test, and Development Commands
- `bun run scripts/dev.ts` — watch/build the frontend into `dist/` for local dev.
- `bun run scripts/build.ts` — production frontend build.
- `bun run scripts/check.ts` — type-check frontend with `tsc`.
- `bun run migrate` — apply SQL migrations to `DATABASE_URL` (defaults to `postgresql://postgres:postgres@localhost:5432/rubhub`).
- `cargo run` — start the backend (serves `/`, `/dist`, `/assets`; requires DB reachable).
- `bun run format` — format/lint frontend/scripts with Biome (pre-commit hook). Use `cargo fmt` (and optionally `cargo clippy`) for Rust.

## Coding Style & Naming Conventions
- Rust: follow `rustfmt` defaults; modules and files use `snake_case`; types/traits `PascalCase`; functions/vars `snake_case`. Keep business logic in `services/`, HTTP wiring in `api.rs`/`auth.rs`.
- Frontend/scripts: Biome enforces tabs and double quotes; prefer `const` + arrow functions; components/files `kebab-case` or `snake_case` to match neighbors. Avoid implicit `any`; favor `zod` for validation.
- Prefer small, composable modules; avoid mixing DB access and request shaping in the same function.

## Testing Guidelines
- Rust: add `#[tokio::test]` where async; run `cargo test` before pushing (ensure a test DB or mocks for SeaORM access). Keep migrations applied so schema matches.
- Frontend: no automated tests yet; at minimum run `bun run check` and manually verify key flows (login, project pages, file serving from `/dist`).

## Commit & Pull Request Guidelines
- Commit messages in this repo are short, descriptive sentences (e.g., “Minor styling and improved permission system”); keep scope focused and avoid large mixed changes.
- PRs should include a clear summary, linked issue (if any), setup steps (env vars, migrations), and screenshots/GIFs for UI-visible changes.
- Note any DB schema updates (new migration file) and whether `bun run build` was run for frontend-impacting changes.

## Security & Configuration Tips
- Required env vars: `DATABASE_URL`, `GIT_ROOT`, `ASSET_ROOT`, `BIND_ADDR` (defaults exist; prefer `.env` in dev). Never commit real credentials or SSH keys; use the sample keys in `data/` only for local testing.
- Server listens on `127.0.0.1:3000` by default and serves `/dist` and `/assets`; ensure those directories exist before deploying. Use strong DB passwords and restrict network exposure when running locally.
