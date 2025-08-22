# Build system overview

This project uses Cargo and npm unified under `cargo-make` as the task runner.
The goal is to keep the developer workflow simple while still producing a single
monolithic binary for production.

## Directory structure

As a reminder, here's the relevant part of the directory structure:

```
src/
├── core/     # Rust core
│   ├── Cargo.toml
│   └── src/...
└── webui/    # WebUI (frontend)
    ├── package.json
    ├── main.js
    ├── src/...
    └── dist/   # compiled static assets
```

- The core program (`src/core/`) is a Rust application that can embed the
  frontend's compiled assets.
- The webUI (`src/webui/`) is a Node.js app that compiles into static files in
  the `src/webui/dist/`.

## Make system

We use [`cargo-make`](https://sagiegurari.github.io/cargo-make/) as a unified
task runner. It is installed together with Rust in the development environment
configuration script.

All build steps (Rust + Node) are defined in a single `Makefile.toml` at the
repo root.

## Tasks

### Development Build

Build, test and run both core (`cargo watch`) and webUI (`npm run dev`) in watch
mode:

```sh
cargo make dev
```

- Hot-reloads webUI and Rust code on changes.
- Suited for local development.

### Production Build

Compile the webUI and embed into the backend to build a release binary:

```sh
cargo make prod
```

Steps performed:

1. `npm ci && npm run build` inside `src/webui` to generate `dist/`.
2. Rust backend (`src/core`) builds with `cargo build --release`.
3. The backend binary embeds `dist/` via
   [`rust-embed`](https://crates.io/crates/rust-embed).

This results in a single binary containing all web assets, which can be deployed
as-is.

### Testing

Run all tests with:

```sh
cargo make test
```

Tasks for testing individual components are also available:

- `core-test`: runs only Rust tests.
- `webui-test`: runs only webUI tests.

### Code Quality and Formatting

We use dedicated tasks to check and fix code quality for both Rust and webUI
code.

#### Code Quality check

Checks formatting and linting for both Rust and webUI. Fails if any issues are
found (does not auto-fix):

```sh
cargo make code-quality
```

Use this in CI to enforce code standards.

#### Auto-fix formatting (for developers)

Automatically fixes formatting issues in both Rust and webUI code:

```sh
cargo make fix
```

- Runs `cargo fmt` (Rust auto-format)
- Runs `npm run format` (webUI auto-format)

Use this locally before committing to ensure code is properly formatted.

### Documentation linting

Markdown documentation is linted with `markdownlint`:

```sh
cargo make doc-lint
# or
cargo make doc-lint-fix
```

### Tasks summary

| Task           | Description                     | Usage              |
| -------------- | ------------------------------- | ------------------ |
| `dev`          | Development build/watch         | Local development  |
| `prod`         | Production build                | Release/deployment |
| `test`         | Run Rust tests                  | Local/CI testing   |
| `code-quality` | Check lint/format (CI)          | CI/code review     |
| `fix`          | Auto-fix formatting (developer) | Local development  |
| `doc`          | Documentation linting           | Local/CI           |

## Developer workflow

- **Rust-only changes**: work inside `src/core`, run `cargo run`.
- **WebUI-only changes**: work inside `src/webui`, run `npm run dev`.
- **Full-stack changes / final builds**: run `cargo make prod`.

```

```
