# Development setup guide

This document outlines the steps to set up a development environment for the
project.

## Prerequisites

- A Linux-based operating system (WSL2 might work too but is not tested).
- `curl` or `wget` for downloading files.
- `git` for version control.
- A text editor or IDE of your choice (e.g. Vim) which you know how to
  configure.
- `docker` 28+ for local development and testing.

## Environment variables

The project uses environment variables to configure various settings across the
application. These variables are defined in a `.env` file located at the root of
the project. An example environment file is provided at `.env.example`. Copy it
to `.env` and adjust values as needed:

```bash
cp .env.example .env
```

## Rust configuration

The project is written in Rust, so you will need to have the Rust toolchain.
This is done through the [`rustup`](https://rustup.rs/) tool, which manages Rust
environments. The [/rust-toolchain.toml](/rust-toolchain.toml) file specifies
the version of Rust to use for the project.

### Setup script

To install `rustup`, the Rust toolchain and recommended tools, run the setup
script:

```bash
./scripts/setup_rust.sh
```

The script will:

- Install `rustup` if it's not already installed.
- Ensure the correct toolchain is active.
- Install recommended Cargo tools.

### Required tools

These are installed automatically and required for development:

- **`rustfmt`**: Code formatter (`cargo fmt`)
- **`clippy`**: Linter for best practices (`cargo clippy`)

We enforce these in CI to guarantee code style and linting consistency.

## Recommended tools

These tools are not strictly required, but highly recommended. Install via:

```bash
cargo install \
  cargo-make \
  cargo-edit \
  cargo-watch \
  cargo-nextest \
  cargo-outdated \
  cargo-udeps \
  cargo-deny \
  cargo-audit \
  cargo-expand \
  concurrently
```

They provide useful functionality for Rust development:

- `cargo-make`: Task runner (like regular `make` but more modern).
- **`cargo-edit`**: Add/remove/upgrade dependencies from CLI.
- **`cargo-watch`**: Auto-run `cargo test`/`cargo run` on file changes.
- **`cargo-nextest`**: Faster, parallel test runner.
- **`cargo-outdated`**: Check for newer dependency versions.
- **`cargo-udeps`**: Detect unused dependencies.
- **`cargo-deny`**: License/security/dependency checks.
- **`cargo-audit`**: Vulnerability scanning against
  [RustSec](https://rustsec.org/).
- **`cargo-expand`**: Expand macros for debugging.
- **`concurrently`**: Run multiple commands concurrently (used in `cargo-make`).

Optional extras for testing and analysis:

- **`cargo-tarpaulin`**: Code coverage (Linux only).
- **`cargo-bloat`**: Analyze binary size.

### Verification

After setup, check that everything is working correctly by running the following

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run
```

If all commands succeed, the environment has been set up correctly.

## Web development configuration

The project makes use of NodeJS for the web UI. To install the required tools,
run the web setup script:

```bash
./scripts/setup_node.sh
```

## Documentation configuration

We make use of Markdown for documentation throughout the project. To install the
required linting and formatting tools, run the docs setup script:

```bash
./scripts/setup_markdown.sh
```

**Note**: you **MUST** install NodeJS through the `setup_node.sh` script first
as this script depends on NodeJS being available.
