# Development Guide

Welcome to the development guide for this project! This document provides all the essential information for setting up
your environment, understanding the development workflow, and contributing effectively.

## Project Overview

This repository contains a Rust-based core application and a web-based UI. The project uses a modular structure, with
the Rust core in `src/core/` and the web UI in `webui/`. CI/CD is managed via GitHub Actions, and builds are coordinated
using `cargo-make` for consistency.

## Prerequisites

- **Operating System**: Ubuntu Linux (WSL2 may work, but is not officially supported)
- **Tools**: `git`, `systemd-nspawn` and a text editor/IDE

## Environment Setup

### 1. Setup Scripts

- To set up the Rust toolchain and required tools, run:
  ```bash
  ./scripts/setup_rust.sh
  ```
- To set up the Node.js environment for the web UI, run:
  ```bash
  ./scripts/setup_node.sh
  ```
- To set up Markdown tooling for documentation, run:
  ```bash
  ./scripts/setup_markdown.sh
  ```

## Development Workflow

### Formatting & Linting

Before pushing code, ensure you follow these steps:

- Check formatting:
  ```bash
  cargo fmt -- --check
  ```
- Run linter:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

### Development server

To run the development server with hot-reloading for the core codebase and web UI, run the following command from the
root of the repo:

```bash
cargo make dev
```

### Building & Testing

To build and test the project, run the following commands from the project root:

- Build the project:
  ```bash
  cargo make build
  ```
- Run tests:
  ```bash
  cargo make test
  ```
- For the web UI, see `webui/README.md` for frontend-specific commands.

### Task Automation

- Use `cargo-make` for common tasks (see `Makefile.toml` for available tasks):
  ```bash
  cargo make <task>
  ```

## CI/CD Pipeline

The project uses GitHub Actions for CI/CD, with workflows defined in `.github/workflows/`:

- **ci.yml**: Linting, formatting, static analysis, tests, and builds. Runs on PRs and pushes to `main`/`dev`.
- **cd.yml**: Release workflow, triggered after successful CI on `main` or manually. Publishes binaries if version is
  valid semver.
- **docs.yml**: Builds and lints documentation. Runs after CI or on docs changes.
- **gh-pages.yml**: Deploys documentation to GitHub Pages.

### Local CI Testing

- You can run the pipeline locally using [`act`](https://github.com/nektos/act`):
  ```bash
  act
  ```

## Best Practices

- **Code Style**: Always run `cargo fmt` and `cargo clippy` before pushing.
- **Commits**: Write clear, concise commit messages using conventional commits.
- **Pull Requests**: Ensure all checks pass before requesting review.
- **Documentation**: Update docs for any user-facing or developer-facing changes.

## Troubleshooting & Resources

- For Rust issues, see [The Rust Book](https://doc.rust-lang.org/book/).
- For CI/CD, check workflow logs in GitHub Actions.
- For environment issues, ensure all prerequisites are installed.

For further help, contact the maintainers or open an issue.
