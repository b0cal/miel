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

## Why?

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

## How?

- Rust🦀 guarantees us memory safety without performance cost
- [tokio🗼](https://tokio.rs/) asynchronous runtime performs efficient async.
  I/O, supports large amount of protocols and has built-in security features
  such as robust timeout handling preventing resource exhaustion.
- [systemd-nspawn](https://wiki.archlinux.org/title/Systemd-nspawn) handles the
  containerization of the services.

> These are the main components used in the project, for a more exhaustive list,
> see the [architecture](/doc/research/architecture.md#rust-libraries)
> description

## 🍯 Build and start

### Pre-requisit

- Debian/Fedora (x86_64)
- systemd-nspawn

### Installation

1. Download the latest release from the [Releases](https://github.com/b0cal/miel/releases) tab

  or

  Clone the project to build from source
 ```sh
git clone https://github.com/b0cal/miel.git

cd miel/src/core

cargo build
```

2. At the first start-up an example config will be generated in `example/template.toml`. Feel free to play with it and use it to start the app
```sh
sudo miel <PATH_TO_CONFIG>
```

>[!INFO]
> super user rights are needed to process the service containers

3. If you enabled the web interface go on http://localhost:3000 to view the dashboard or use the documented [web API](./docs/development/web_api.md) to fetch the data collected for further analysis


>[!EXAMPLE]
> Get all sessions basic data
> ```sh
> wget http://localhost:3000/api/sessions
> ```
> 
> Get session data by id
> ```sh
> wget http://localhost:3000/api/sessions/:id/data)
> ```
>
> Get sessions artifact by id
> ```sh
> wget http://localhost:3000/api/sessions/:id/artifacts)
> ```


## Known issues

- If the web application crashes it panics and stops the application
- If the app is stopped and restarted too fast the binding ports could be unavailable for a bit.
  - **Workaround**: Just wait for the timeout for the port to be available again (approx. 1 min)

## Further improvements

- Support [oci](opencontainers.org) container images
- Allow UDP based services
- Allow hotswapping services from the dashboard
- More comprehensive filtering solution on the dashboard

## Contributing

Please see [CONTRIBUTING](https://github.com/b0cal/miel/contributing) tab.
