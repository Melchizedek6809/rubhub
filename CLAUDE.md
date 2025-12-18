# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RubHub is a federated git forge written in Rust with a TypeScript frontend. It uses git itself as the storage backend (no external database) and compiles to a single executable that handles both HTTP and SSH servers.

**Key Philosophy**: Git as the single source of truth. Everything is stored as JSON files within the git directory structure, enabling federation through git remotes.

## Development Commands

### Frontend Development (Node/npm)

```bash
npm run dev          # Watch mode - rebuilds frontend on changes
npm run build        # Production build - minified frontend assets
npm run check        # TypeScript type checking
npm run format       # Format frontend code with Biome
npm run format:unsafe # Format with unsafe fixes
```

### Backend Development (Rust)

```bash
cargo build          # Build the Rust binary
cargo build --release # Production build
cargo run            # Run the development server
cargo test           # Run all tests (integration tests in tests/)
cargo clippy         # Lint Rust code
cargo fmt            # Format Rust code
```

### Running Tests

```bash
# Run all integration tests
cargo test

# Run a specific test
cargo test basic_workflow

# Run with output
cargo test -- --nocapture
```

## Architecture Overview

### Monorepo Structure

- **Rust backend** (`src/`): Axum HTTP server + russh SSH server in single executable
- **Frontend** (`frontend/app/`): TypeScript web application, built with Vite
- **Templates** (`templates/`): Askama server-side templates
- **Data directory** (`data/`): Git repositories and session storage (not in repo)

### Backend Architecture (Rust)

**Entry Point Flow:**
- `src/main.rs` → `src/lib.rs::run()` → spawns HTTP and SSH servers
- Uses **single-threaded Tokio runtime** for resilience (intentional design choice)

**Key Modules:**
- `http.rs` - Axum router and middleware setup
- `ssh.rs` - SSH server implementation using russh
- `state.rs` - AppConfig and GlobalState (shared between servers)
- `controllers/` - MVC-style route handlers (auth, project, user)
- `models/` - Data structures (User, Project) with JSON persistence
- `services/` - Business logic (session, repository, password, validation)
- `extractors/` - Custom Axum path extractors for typed URL parameters
- `views/` - Response rendering with theme injection system

**Data Persistence Pattern:**
- **No database**: All data stored as JSON files
- Users: `data/git/!{username}.json` (special `!` prefix for metadata)
- Projects: `data/git/{username}/!{project_slug}.json`
- Sessions: `data/sessions/{session_id}.json`
- Git repos: Bare repositories at `data/git/{username}/{project_slug}`

**Request Path Extraction:**
Custom extractors (`PathUser`, `PathUserProject`, etc.) automatically load entities from storage. Uses `~` prefix for user paths: `/~{username}` → loads User, `/~{username}/{project}` → loads User + Project. Returns 404 if loading fails.

**Access Control:**
```rust
pub enum AccessType {
    None,   // No access
    Read,   // Read-only
    Write,  // Read + write
    Admin,  // Full control (owner only)
}
```

### Frontend Architecture

**Technology**: TypeScript + Vite bundler

**Build Output**: Vite builds `frontend/app/app.html` → `dist/` directory → embedded in Rust binary via `rust-embed`

**Theme Injection System:**
1. Server renders Askama template with placeholders: `<!--HEAD-->` and `<!--BODY-->`
2. Compiled `app.html` contains theme/styling
3. Server injects rendered content into theme placeholders
4. Result: Server-rendered HTML with client-side interactivity

**Client-Side Features:**
- Session detection via `session_user` cookie (JSON)
- Branch switcher dropdown
- Sidebar toggle for mobile
- Progressive enhancement (works without JS)

### Configuration

Environment variables (use `.env` or `.env.development`):
- `SITE_URL` - Base URL for the site (e.g., `http://localhost:3000`)
- `SSH_PUBLIC_URL` - Public SSH host for clone URLs
- `DIR_ROOT` - Data directory root (defaults to `./data/`)
- `HTTP_BIND_ADDR` - HTTP server bind address (default: `127.0.0.1:3000`)
- `SSH_BIND_ADDR` - SSH server bind address (default: `127.0.0.1:2222`)

## Important Patterns

### Adding New Routes

1. Add handler in appropriate `controllers/` module
2. Register route in `src/http.rs`
3. Use custom extractors for path parameters (`PathUser`, `PathUserProject`)
4. Create Askama template in `templates/`
5. Implement `ThemedRender` trait for response

### Working with Git Repositories

Use the `services/repository.rs` module which wraps `gix` crate. All repository operations go through this service layer.

### Session Management

Sessions are file-based with 30-day expiration:
- `session_id` cookie: HTTP-only, secure
- `session_user` cookie: Readable by client JS (JSON with user data)

### Validation

Use functions from `services/validation.rs`:
- `validate_username()` - Strict username rules
- `validate_slug()` - URL-safe slugs
- `slugify()` - Convert names to slugs

### Code Style

**Frontend** (enforced by Biome):
- Tab indentation
- Double quotes for strings
- Auto-organize imports

**Backend** (use `cargo fmt`):
- Standard Rust formatting
- Follow existing patterns in codebase

## Testing

Integration tests use a temporary directory and spin up the full server. The `with_backend()` helper manages lifecycle:

```rust
#[tokio::test(flavor = "current_thread")]
async fn my_test() {
    with_backend(async {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap();

        // Test code here
    }).await;
}
```

Tests use fixed ports (32323 for HTTP, 32324 for SSH) to avoid conflicts.

## Build Pipeline

1. **Frontend**: Vite builds HTML/TS/CSS → `dist/` directory
2. **Backend**: Rust embeds `dist/` assets using `rust-embed` crate
3. **Output**: Single executable with embedded frontend assets

The `rust-embed` crate serves assets at runtime from the binary, no external files needed.

## Git Workflow

This repo follows a standard git workflow. The default branch is `main`.
