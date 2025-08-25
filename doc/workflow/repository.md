# Repository structure

The project follows a modular monolithic design but the source code is organized
into logical subsystems under the `src/` directory. This makes it easy to
navigate, extend, and eventually refactor into separate services if needed.

At the top level, the repository contains documentation (`doc/`), helper scripts
(`scripts/`), licensing, and toolchain configuration.

The source code is stored in the `src/` directory:

- **`core/`**: Rust code for the main application logic and subsystems.
- **`webui/`**: Frontend code for the web interface.

## Core

All Rust code lives in the `src/core/` folder. The entrypoint is
`src/core/main.rs`, which handles the core subsystems and launches the
controller.

Each subsystem is placed in its own
[module](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)
directory under `src/core/`:

- **`controller/`**: main application logic and orchestration of components.
- **`config/`**: loading and validating configuration files.
- **`listener/`**: network entrypoints.
- **`session/`**: manages per-connection state, etc.
- **`container/`**: integrates with `systemd-nspawn` for sandboxed execution.
- **`recorder/`**: session capture.
- **`storage`**: data persistence (database).
- **`cli/`**: CLI logic, including subcommands and argument parsing.
- **`api/`**: REST endpoints for programmatic access and webUI backend.
- **`util/`**: shared utilities such as logging, metrics, and error handling.

This organization allows the codebase to remain clean and maintainable. Each
subsystem is self-contained, making it straightforward to test, replace, or
expand.

## WebUI

The web interface code lives in `src/webui/`.
