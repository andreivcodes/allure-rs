# Allure-RS

A comprehensive Rust library for generating [Allure](https://allurereport.org/) test reports with full feature parity to allure-js-commons.

## Features

- **Test metadata annotations** - epic, feature, story, severity, owner, tags
- **Test steps** - nested step support with timing
- **Attachments** - text, JSON, binary files
- **BDD-style steps** - given, when, then, and, but
- **Links** - issue tracker and test management system links
- **Flaky/muted test support**
- **Environment and categories configuration**
- **Async test support** (tokio-first)
- **Framework agnostic** - works with `#[test]`, `tokio::test`, `rstest`

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
allure = "0.1"
```

## Quick Start

```rust
use allure::prelude::*;

// Note: Metadata attributes must come BEFORE #[allure_test]
#[epic("User Management")]
#[feature("Authentication")]
#[severity(Severity::Critical)]
#[allure_test]
fn test_login() {
    step("Initialize user", || {
        // setup code
    });

    step("Perform login", || {
        // test code
        assert!(true);
    });

    attachment::text("Debug info", "Login successful");
}
```

## BDD Style

```rust
use allure::prelude::*;

#[allure_test]
fn test_user_registration() {
    let user = bdd::given("a new user with valid email", || {
        User::new("test@example.com")
    });

    bdd::when("the user submits registration form", || {
        user.register();
    });

    bdd::then("the user account should be created", || {
        assert!(user.is_registered());
    });
}
```

## Configuration

```rust
use allure::configure;

// Initialize before running tests
configure()
    .results_dir("allure-results")
    .clean_results(true)
    .init()
    .unwrap();
```

## Environment Info

```rust
use allure::environment;

environment()
    .set("rust_version", env!("CARGO_PKG_RUST_VERSION"))
    .set("os", std::env::consts::OS)
    .set_from_env("CI", "CI")
    .write()
    .unwrap();
```

## Supported Annotations

**Important:** Metadata annotations must be placed BEFORE `#[allure_test]` due to Rust's proc macro processing order.

| Annotation | Purpose |
|------------|---------|
| `#[epic("...")]` | Top-level business capability |
| `#[feature("...")]` | Feature under epic |
| `#[story("...")]` | User story under feature |
| `#[suite("...")]` | Test suite grouping |
| `#[severity(Severity::Critical)]` | Test importance |
| `#[owner("...")]` | Test maintainer |
| `#[tag("...")]` | Arbitrary tags |
| `#[id("...")]` | Test case ID |
| `#[description("...")]` | Markdown description |
| `#[issue("...")]` | Link to issue tracker |
| `#[tms("...")]` | Link to test management |
| `#[flaky]` | Mark test as flaky |
| `#[muted]` | Mute test results |

## Viewing Reports

After running tests, generate the HTML report using the built-in cargo commands:

```bash
# Run tests and generate report
cargo allure

# Generate report from existing results (skip tests)
cargo allure-report

# Open report in browser
cargo allure-open
```

Or use the Allure CLI directly:

```bash
# Run tests
cargo test

# Generate report
allure generate allure-results -o allure-report

# Open report
allure open allure-report
```

## Crate Structure

- `allure` - Main facade crate (re-exports everything)
- `allure-core` - Core types, model, and runtime
- `allure-macros` - Procedural macros

## License

MIT OR Apache-2.0
