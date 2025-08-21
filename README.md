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

**miel is a modular honeypot adapting to attackers interactions**

- Expose voluntarily vulnerable services to analyze attackers behavior
- Let miel adapt to the attacker's request to serve him with the right service
- Simply add new services with configuration files
- Link a database to store paquet trace, shell interactions, metadata, etc.
- Ships with pre-filled ssh and http configuration files

### Why?
Honeypots can be used in two situations. First to deceive attackers and avoid real infrastructure to be compromised. Secondly to intercept and retain attacker's connections in a MiTM way in order to analyze and collect interactions, IoC or payloads. 

Today's available solutions allow to either masquerade one service at a time or deploy multiple honeypots, each one masquerading one service, upon completing a full scan of the real infrastructure to detect which systems are present and need to be secured.

***miel*** seeks to deliver a chameleon research honeypot. One capable of serving the corresponding service that matches the attacker's expectations, providing richer interaction data for analysis.

### How?

- Rust🦀 guarantees us memory safety without performance cost 
- [tokio🗼](https://tokio.rs/) asynchronous runtime performs efficient async I/O, supports large amount of protocols and has built-in security features such as robust timeout handling preventing resource exhaustion
- [systemd-nspawn](https://wiki.archlinux.org/title/Systemd-nspawn) handles the containerization of the services

> These are the main components used in the project, for a more exhaustive list, see the [architecture](/doc/research/architecture.md#rust-libraries) description

## Contributing

Please see CONTRIBUTING tab
