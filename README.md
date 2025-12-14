# Allure-RS

[![CI](https://github.com/andreivcodes/allure-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/andreivcodes/allure-rs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/allure-rs.svg)](https://crates.io/crates/allure-rs)
[![Documentation](https://docs.rs/allure-rs/badge.svg)](https://docs.rs/allure-rs)
[![MSRV](https://img.shields.io/badge/MSRV-1.75-blue.svg)](https://github.com/andreivcodes/allure-rs)
[![License](https://img.shields.io/crates/l/allure-rs.svg)](LICENSE)

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
allure-rs = "0.1"
```

## Requirements

- **Rust 1.75 or higher** (MSRV)
- **Allure CLI** (for viewing reports) - [Installation Guide](https://allurereport.org/docs/install/)

## Quick Start

```rust
use allure_rs::prelude::*;

// Note: Metadata attributes must come BEFORE #[allure_test]
#[allure_epic("User Management")]
#[allure_feature("Authentication")]
#[allure_severity("critical")]
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
use allure_rs::prelude::*;

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
use allure_rs::configure;

// Initialize before running tests
configure()
    .results_dir("allure-results")
    .clean_results(true)
    .init()
    .unwrap();
```

## Environment Info

```rust
use allure_rs::environment;

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
| `#[allure_epic("...")]` | Top-level business capability |
| `#[allure_epics("...", "...")]` | Multiple epics |
| `#[allure_feature("...")]` | Feature under epic |
| `#[allure_features("...", "...")]` | Multiple features |
| `#[allure_story("...")]` | User story under feature |
| `#[allure_stories("...", "...")]` | Multiple stories |
| `#[allure_suite_label("...")]` | Test suite grouping |
| `#[allure_parent_suite("...")]` | Parent suite grouping |
| `#[allure_sub_suite("...")]` | Sub-suite grouping |
| `#[allure_severity("...")]` | Test importance (blocker/critical/normal/minor/trivial) |
| `#[allure_owner("...")]` | Test maintainer |
| `#[allure_tag("...")]` | Single tag |
| `#[allure_tags("...", "...")]` | Multiple tags |
| `#[allure_id("...")]` | Test case ID |
| `#[allure_title("...")]` | Custom test title |
| `#[allure_description("...")]` | Markdown description |
| `#[allure_description_html("...")]` | HTML description |
| `#[allure_issue("...")]` | Link to issue tracker |
| `#[allure_tms("...")]` | Link to test management |
| `#[allure_link("...")]` | Generic link |
| `#[allure_flaky]` | Mark test as flaky |

## Viewing Reports

After running tests, generate the HTML report using the [Allure CLI](https://allurereport.org/docs/install/):

```bash
# Run tests
cargo test

# Generate report
allure generate allure-results -o allure-report

# Open report in browser
allure open allure-report
```

## Runtime API

In addition to macros, you can use the runtime API for dynamic metadata:

```rust
use allure_rs::prelude::*;

#[allure_test]
fn test_with_runtime_api() {
    // Set metadata dynamically
    epic("User Management");
    feature("Authentication");
    severity(Severity::Critical);
    owner("team@example.com");

    // Add parameters
    parameter("browser", "Chrome");
    parameter("version", "120.0");

    // Create steps
    step("Login step", || {
        // test code
    });

    // Add attachments
    attachment::text("Response", r#"{"status": "ok"}"#);
}
```

## Async Tests

Works with `tokio::test` and other async test frameworks:

```rust
use allure_rs::prelude::*;

#[allure_epic("API")]
#[allure_test]
#[tokio::test]
async fn test_async_api() {
    step("Make request", || {
        // test code
    });
}
```

Enable the `async` feature for async step support:

```toml
[dev-dependencies]
allure-rs = { version = "0.1", features = ["async"] }
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `async` | Enable async step support with `futures` crate |
| `tokio` | Enable tokio task-local storage for async tests |

## Crate Structure

- `allure-rs` - Main facade crate (re-exports everything)
- `allure-core` - Core types, model, and runtime
- `allure-macros` - Procedural macros

## License

MIT OR Apache-2.0
