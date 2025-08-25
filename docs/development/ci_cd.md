# DevOps Pipeline Design

## Overview

This document describes the CI/CD pipeline design for the application. We use
[GitHub Actions](https://docs.github.com/en/actions/get-started/understand-github-actions)
to automate:

1. Code quality checks (linting, formatting, security scanning)
2. Automated builds (Rust core + webUI)
3. Automated tests
4. Artifact packaging and release

The workflows make heavy use of the `cargo-make` system to ensure consistent
builds across the local development and CI environments.

## Goals

- **Fast feedback**: run Rust and NodeJS tests on every pull request.
- **Quality**: enforce consistent code formatting and linting standards.
- **Audit**: perform static analysis and dependency vulnerability scans.
- **Repeatable builds**: deterministic builds using GitHub runners.
- **Deployment ready artifacts**: automatically generate tagged binary releases
  on GitHub Releases.

## Workflow

The CI/CD pipeline is implemented in two workflow file:

- **`.github/workflows/ci.yml`** (CI workflow): lint, format, static analysis,
  tests, builds, and releases. Triggered on pull requests and pushes to `main`
  and `dev` branches, with special handling for version tags. This workflow does
  **NOT** run on changes to documentation files only (`/doc` directory). If the
  workflow runs on the `main` branch, it will also create a tag based on the
  semantic versioning in `Cargo.toml`.
- **`.github/workflows/release.yml`** (CD workflow): triggered on semver tag
  pushes. Downloads the built binary artifact from the CI workflow and publishes
  it to GitHub Releases.

### CI workflow

1. **Code quality and audit**
   - Run linting and formatting checks for Rust, JavaScript and Markdown.
   - Performs dependency audits with `cargo-audit`.
   - Perform static analysis with CodeQL for security vulnerabilities in Rust
     and JavaScript code.
2. **Test**
   - Run Rust unit tests for the core application.
   - Run NodeJS tests for the webUI.
3. **Build**
   - Build the webUI frontend.
   - Embed webUI assets into the Rust core binary.
   - Produce a single self-contained binary for validation.
   - Upload built binaries as a GitHub Actions artifact.
4. **Tag** (conditional)
   - Only runs on pushes to the `main` branch:
     - Create a new git tag based on the semantic versioning in `Cargo.toml`.

### CD workflow

The release process is triggered by on pushed to main and following the semantic
versioning convention:

```
v<major>.<minor>.<patch>
e.g., v1.0.0, v1.1.3
```

Tags are automatically extracted from the `Cargo.toml` file during the build.

The release workflow performs the following steps:

1. **Build**

- Build the application with a production profile.

2. **Release**

- Create a new GitHub Release with the tag name and description.

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
