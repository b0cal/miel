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

Run Rust tests:

```sh
cargo make test
```

## Developer workflow

- **Rust-only changes**: work inside `src/core`, run `cargo run`.
- **WebUI-only changes**: work inside `src/webui`, run `npm run dev`.
- **Full-stack changes / final builds**: run `cargo make prod`.
