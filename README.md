<div align="center">
  <pre>
██████╗  ██████╗  ██████╗ █████╗ ██╗         ██╗███╗   ███╗██╗███████╗██╗     
██╔══██╗██╔═████╗██╔════╝██╔══██╗██║        ██╔╝████╗ ████║██║██╔════╝██║     
██████╔╝██║██╔██║██║     ███████║██║       ██╔╝ ██╔████╔██║██║█████╗  ██║     
██╔══██╗████╔╝██║██║     ██╔══██║██║      ██╔╝  ██║╚██╔╝██║██║██╔══╝  ██║     
██████╔╝╚██████╔╝╚██████╗██║  ██║███████╗██╔╝   ██║ ╚═╝ ██║██║███████╗███████╗
╚═════╝  ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝    ╚═╝     ╚═╝╚═╝╚══════╝╚══════╝
  </pre>
</div>

## 🍯 About

**`miel` is a modular honeypot software that adapts to attackers interactions.**

- Expose voluntarily vulnerable services to analyze attackers behavior.
- Let miel adapt to the attacker's request to serve him with the right service.
- Simply add new services with configuration files.
- Link a database to store paquet trace, shell interactions, metadata, etc.
- Ships with pre-filled ssh and http configuration files.

### Why?

Honeypots can be used in two situations. First to deceive attackers and avoid
real infrastructure to be compromised. Secondly to intercept and retain
attacker's connections in a MiTM way in order to analyze and collect
interactions, IoC or payloads.

Today's available solutions allow to either masquerade one service at a time or
deploy multiple honeypots, each one masquerading one service, upon completing a
full scan of the real infrastructure to detect which systems are present and
need to be secured.

**_miel_** seeks to deliver a chameleon research honeypot. One capable of
serving the corresponding service that matches the attacker's expectations,
providing richer interaction data for analysis.

### How?

- Rust🦀 guarantees us memory safety without performance cost
- [Tokio🗼](https://tokio.rs/) asynchronous runtime performs efficient async.
  I/O, supports large amount of protocols and has built-in security features
  such as robust timeout handling preventing resource exhaustion.
- [`systemd-nspawn`](https://wiki.archlinux.org/title/Systemd-nspawn) handles
  the containerization of the services.

> These are the main components used in the project, for a more exhaustive list,
> see the [architecture](/doc/research/architecture.md#rust-libraries)
> description

## 🍯 Usage

### Prerequisites

- A `x64` Debian based OS (also works on Fedora)
- `systemd-nspawn` (installable with `sudo apt install systemd-nspawn`)
- NodeJS version 22+
- Rust version 1.89

If you need to install these dependencies, follow
[the development guide](https://github.com/b0cal/miel/tree/main/DEVELOPEMENT.md)

### Configuration

The configuration file is in TOML format. A sample configuration file is
available in `/example/config/config.toml`. All modifiable parameters are
documented there.

Example service configurations are available at
[https://github.com/b0cal/miel/tree/main/example/config/services](https://github.com/b0cal/miel/tree/main/example/config/services).

Alternatively, some environment variables are available. These take precedence
over file-based configuration. The variables are the following:

```txt
RUST_LOG=info
MIEL_STORAGE_PATH=./storage
SERVICE_DIR=./services
```

A complete `miel` command to run the program from the `src/core` with
environment variables might look something like this:

```sh
RUST_LOG=debug \
SERVICE_DIR=../../example/config/services \
sudo target/release/miel ../../example/config/config.toml
```

### Installation

Ensure the prerequisites are met, then either download a release or build from
source.

#### From GitHub Releases

1. Download the latest release from the
   [Releases](https://github.com/b0cal/miel/releases) tab.
2. Fetch the
   [default configuration](https://github.com/b0cal/miel/blob/main/example/config/config.toml)
   from the repository.

#### Build from source

1. Clone the project to build from source
   ```sh
   git clone https://github.com/b0cal/miel.git
   cd miel
   cargo make prod
   ```
   The executable can then be found at `/src/core/target/release/miel`
2. The default configuration is available in `/example/config/config.toml`

### Running `miel`

> [!NOTE]
> super user rights are needed to process the service containers

```sh
sudo miel <PATH_TO_CONFIG>
```

Then navigate to [http://localhost:3000](http://localhost:3000) to view the web
interface. The API is available at
[http://localhost:3000/api](http://localhost:3000/api).

> [!TIP]
> **EXAMPLES:**
> Get all sessions basic data
>
> ```sh
> wget http://localhost:3000/api/sessions
> ```
>
> Get session data by id
>
> ```sh
> wget http://localhost:3000/api/sessions/:id/data)
> ```
>
> Get sessions artifact by id
>
> ```sh
> wget http://localhost:3000/api/sessions/:id/artifacts)
> ```

## 💻 Development

See [DEVELOPMENT.md](DEVELOPMENT.md) and refer to the documentation in `/docs`.

## 🔨 Contributing

Please see
[CONTRIBUTING](https://github.com/b0cal/miel?tab=contributing-ov-file) tab.

## 🚩 Known issues

- If the web application crashes it panics and stops the application
- If the app is stopped and restarted too fast the binding ports could be
  unavailable for a bit.
  - **Workaround**: Just wait for the timeout for the port to be available again
    (approx. 1 min)

## 🛣️ Further improvements

- Support [OCI](opencontainers.org) container images
- Enhance support for UDP based services
- Control services from the dashboard
- Implement a comprehensive filtering solution on the dashboard
