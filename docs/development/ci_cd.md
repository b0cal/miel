# DevOps Pipeline Design

## Overview

This document describes the CI/CD pipeline design for the application. We use
[GitHub Actions](https://docs.github.com/en/actions/get-started/understand-github-actions)
to automate:

1. Code quality checks (linting, formatting, security scanning)
2. Automated builds (Rust core + webUI)
3. Automated tests
4. Artifact packaging and release
5. Documentation build and deployment

The workflows make heavy use of the `cargo-make` system to ensure consistent
builds across the local development and CI environments.

## Goals

- **Fast feedback**: run Rust and NodeJS tests on every pull request.
- **Quality**: enforce consistent code formatting and linting standards.
- **Audit**: perform static analysis and dependency vulnerability scans.
- **Repeatable builds**: deterministic builds using GitHub runners.
- **Deployment ready artifacts**: automatically generate tagged binary releases
  on GitHub Releases.
- **Up-to-date documentation**: automatically build and deploy documentation to GitHub Pages.

## Workflows Overview

The CI/CD pipeline is implemented using four workflow files:

- **`.github/workflows/ci.yml`** (CI workflow): Lint, format, static analysis, tests, and builds. Triggered on pull requests and pushes to `main` and `dev` branches (excluding documentation-only changes), and manual dispatch.
- **`.github/workflows/cd.yml`** (CD workflow): Handles release logic. Triggered after a successful CI run on the `main` branch or via manual dispatch. Checks for a valid semver version in `Cargo.toml` before releasing.
- **`.github/workflows/docs.yml`** (Docs workflow): Builds and lints documentation. Triggered after CI completes on `main`, on PRs/pushes that touch `docs/` or `src/core/`, and manual dispatch. Uses a path filter to only run jobs if documentation is changed.
- **`.github/workflows/gh-pages.yml`** (GitHub Pages workflow): Deploys documentation to GitHub Pages. Triggered after the Docs workflow completes on `main`, on pushes to `gh-pages`, and manual dispatch.

### Workflow Structure

#### CI Workflow (`ci.yml`)

1. **Code quality and audit**
   - Run linting and formatting checks for Rust, JavaScript, and Markdown.
   - Perform dependency audits with `cargo-audit`.
   - (If enabled) Perform static analysis with CodeQL for security vulnerabilities.
2. **Test**
   - Run Rust unit tests for the core application.
   - Run NodeJS tests for the webUI.
3. **Build**
   - Build the webUI frontend.
   - Embed webUI assets into the Rust core binary.
   - Produce a single self-contained binary for validation.
   - Upload built binaries as a GitHub Actions artifact.

#### CD Workflow (`cd.yml`)

- Triggered after a successful CI workflow run on the `main` branch or via manual dispatch.
- Checks that the version in `src/core/Cargo.toml` is valid semver (MAJOR.MINOR.PATCH).
- If valid, proceeds with release steps (publishing binaries, etc.).

#### Docs Workflow (`docs.yml`)

- Triggered after CI completes on `main`, on PRs/pushes that touch `docs/` or `src/core/`, and manual dispatch.
- Uses a path filter to only run jobs if documentation is changed.
- Runs linting and build steps for documentation.

#### GitHub Pages Workflow (`gh-pages.yml`)

- Triggered after the Docs workflow completes on `main`, on pushes to `gh-pages`, and manual dispatch.
- Deploys the built documentation to GitHub Pages.

## Release Process

The release process is handled by the **CD workflow** (`cd.yml`), which is triggered after a successful CI run on the `main` branch or via manual dispatch. The workflow checks that the version in `src/core/Cargo.toml` follows semantic versioning (MAJOR.MINOR.PATCH). If valid, it proceeds with the release steps, such as publishing binaries to GitHub Releases under the corresponding version tag.

## Local Testing

Developers can run the pipeline locally using
[`act`](https://github.com/nektos/act):

1. Install `act`:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash
   ```

2. Simulate workflows locally:

   ```bash
   act pull_request  # Simulate a PR workflow
   act push          # Simulate a push workflow
   act -j build      # Run a specific job
   ```

This allows contributors to test pipeline changes without committing to GitHub.

## Future Enhancements

- Add integration and performance testing.
- Collect code coverage metrics and publish reports.
- Automate deployments to production environment.
- Add Teams notifications for release events and pipeline failures.
