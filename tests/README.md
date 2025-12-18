# Integration Tests

This directory contains integration tests for RubHub.

## Structure

```
tests/
├── common/
│   └── mod.rs          # Shared test utilities
├── integration_tests.rs # Basic workflow tests
├── project_tests.rs     # Project-related tests
└── README.md           # This file
```

## Shared Utilities (`tests/common/mod.rs`)

### `with_backend(test_fn)`
Runs a test with a temporary RubHub backend instance.
- Creates isolated temporary directory
- Binds to OS-assigned ports (enables parallel tests)
- Passes `GlobalState` to your test function
- Automatically cleans up after test

**Example:**
```rust
with_backend(|state| async move {
    let base_url = &state.config.base_url;
    let client = test_client();

    // Your test code here
}).await;
```

### `test_client()`
Creates a reqwest HTTP client with cookie store enabled.

**Example:**
```rust
let client = test_client();
let response = client.get(format!("{base_url}/")).send().await?;
```

### `response_contains(client, url, needle)`
Helper to assert that an HTTP response contains specific text.

**Example:**
```rust
response_contains(&client, &format!("{base_url}/"), "RubHub")
    .await
    .unwrap();
```

## Adding New Tests

### Create a new test file:

1. Create `tests/my_tests.rs`
2. Import the common module: `mod common;`
3. Import utilities: `use common::{with_backend, test_client, response_contains};`
4. Write your tests

**Example (`tests/my_tests.rs`):**
```rust
mod common;

use common::{test_client, with_backend, response_contains};

#[tokio::test(flavor = "current_thread")]
async fn my_test() {
    with_backend(|state| async move {
        let base_url = &state.config.base_url;
        let client = test_client();

        // Test code here
        response_contains(&client, &format!("{base_url}/"), "text")
            .await
            .unwrap();
    })
    .await;
}
```

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test integration_tests

# Run specific test
cargo test basic_workflow

# Run with parallel execution (default)
cargo test -- --test-threads=4

# Run with output
cargo test -- --nocapture
```

## Test Features

- ✅ **Parallel execution**: Tests use OS-assigned ports (no conflicts)
- ✅ **Isolated environments**: Each test gets its own temp directory
- ✅ **Full state access**: Tests receive `GlobalState` with all config
- ✅ **Cookie-aware**: HTTP client maintains sessions automatically
- ✅ **Fast cleanup**: Temporary directories are removed after tests

## Available via `GlobalState`

Your test functions receive a `GlobalState` with access to:

```rust
state.config.base_url          // HTTP server URL (e.g., "http://127.0.0.1:45678")
state.config.ssh_public_host   // SSH server host:port
state.config.git_root          // Path to git repositories
state.config.dir_root          // Test data root directory
// ... and all other config fields
```

## Tips

1. **Use `#[tokio::test(flavor = "current_thread")]`** - Matches production runtime
2. **Enable cookie store** - The `test_client()` helper does this automatically
3. **Use format strings** - Build URLs dynamically: `format!("{base_url}/path")`
4. **Check the output** - Use `--nocapture` to see println! statements
5. **Parallel-safe** - All tests can run simultaneously without conflicts
