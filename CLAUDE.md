# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RubHub is a federated Git forge written in Rust. It stores everything in Git (no database), handles both HTTP and SSH protocols for repository operations, and compiles to a single executable with embedded frontend assets.

## Build & Development Commands

```bash
# Frontend (TypeScript → JavaScript via Vite)
npm run build          # Production build to dist/
npm run dev            # Watch mode for development
npm run check          # TypeScript type checking
npm run format         # Format with Biome

# Backend (Rust)
cargo build            # Build the binary
cargo test             # Run tests
cargo clippy           # Run clippy lints

# Development with bacon (watch mode)
bacon                  # Default: runs the server with auto-restart
bacon test             # Watch tests
bacon clippy-all       # Watch clippy on all targets
```

Build frontend before running the Rust server (assets are embedded at compile time via rust-embed).

## Architecture

### Dual Protocol Server
- **HTTP**: Axum-based web interface and Git HTTP smart protocol
- **SSH**: Russh-based server handling `git push/pull` directly (no system SSH needed)

### Key Directories
- `src/controllers/` - HTTP route handlers organized by feature
- `src/services/` - Business logic (Git operations, sessions, markdown)
- `src/models/` - Data structures
- `src/extractors/` - Axum path parameter extractors
- `templates/` - Askama HTML templates (compiled to Rust)
- `auth_store/` - Workspace member for authentication (in-memory DashMap + filesystem)
- `frontend/` - TypeScript source (minimal: branch switcher, syntax highlighting)

### Data Storage (Git as Database)
- User repos: `DIR_ROOT/git/{username}/{project_slug}/` (bare repos)
- Project metadata: YAML frontmatter in orphan `rubhub/info` branch
- Issues: Directory structure within repos with YAML frontmatter
- Sessions: File-based at `DIR_ROOT/sessions/`

### Shared State Pattern
`GlobalState` (in `src/state.rs`) holds `AuthStore` and `AppConfig`, passed to all handlers via Axum's `.with_state()`.

## Environment Configuration

Copy `.env.example` to `.env`. Key variables:
- `DIR_ROOT` - Data directory for repos and sessions
- `BASE_URL` - Public URL for the instance
- `HTTP_BIND_ADDRESS` / `SSH_BIND_ADDRESS` - Server bind addresses
- `CSRF_SECRET` - Secret for CSRF protection

## Testing

```bash
cargo test                           # All tests
cargo test test_name                 # Single test
cargo test -- --nocapture            # Show output
```

Integration tests use `reqwest` with actual HTTP requests.
