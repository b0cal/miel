# Software Architecture Overview

The software adopts a controller-manager architecture. The components are the
following:

1. **Controller**

   - Single process that loads configs (both global and per-service), drives
     managers, monitors services health, and coordinates other components.
   - Responsible for policy, lifecycle (start/stop), logging, metrics, and
     scheduling container creation.

2. **Listener Manager**

   - Binds/advertises ports on the host (TCP/UDP) and dispatches incoming
     connections to Session Manager.
   - Each configured service has a logical listener (port + protocol handler).
   - Forwards the connection to the Session Manager with appropriate metadata.

3. **Session Manager**

   - For each incoming connection, creates a session object and spawns an
     ephemeral container associated to the IP address of the session, and with
     an appropriate timeout before destruction.
   - Creates a PTY pair or pipes to capture stdio streams.

4. **Container Manager (systemd-nspawn integration)**

   - Responsible for spinning up ephemeral `systemd-nspawn` machines and
     managing a queue of pre-created containers.
   - Handles containers lifecycle and resource management.
   - Applies hardening: capability drops, seccomp, cgroups, no host mounts,
     egress filtering.
   - Applies obfuscation layer: applies modifications to the container to make
     it mimic an actual production service.
   - Provides mechanisms to attach a container to a TCP/UDP connection.

5. **Recorder**

   - Captures session metadata (src IP, src port, dst port, timestamps,
     protocol), packets, session logs, PTY transcript (stdin/out), environment
     (container id, image, config).
   - Sends structured artifacts to storage layer.

6. **Storage Manager**

   - Modular system for storing captured artifacts and the associated metadata.
   - NoSQL support during initial development.

7. **API / WebUI**

   - REST API for viewing status, sessions and downloading captures.
   - Only on the local network and without authentication.

## Data flow for single connection

1. `Controller` loads configurations. `Listener Manager` binds ports.
2. `Container Manager` starts container from configs
3. TCP connection arrives -> Listener accepts connection and allocates session.
4. `Session Manager` instructs Container Manager to spawn or attach to
   container.
5. `Container Manager` returns an attach point.
6. `Session Manager` connects the socket to the container while eavesdropping.
7. `Recorder` records the connection's stdio, packets and metadata until
   disconnection or inactivity timeout, then sends artifacts to storage.
8. `Session Manager` instructs `Container Manager` to clean up the container
   after specified TTL.

## Configuration

```toml
# config.toml
[global]
bind_address = "0.0.0.0"
log_level = "debug"
log_dir = "/var/lib/honeypot/logs" # Or default to /var/log/honeypot
max_sessions = 500

[[service]]
name = "fake-ssh"
port = 22
protocol = "tcp"
container_template = "ssh-template" # A nspawn-ready image
capture = { pty = true, pcap = true, metadata = true }
session = { timeout_seconds = 600, max_bytes = 10485760 }

[[service]]
name = "fake-http"
port = 80
protocol = "tcp"
container_template = "http-template"
capture = { pty = false, pcap = true, metadata = true }
```

## Rust libraries

Useful Rust crates:

- `tokio` for async runtime
- `serde`, `serde_json`, `toml` for config
- `tracing`, `tracing-subscriber` for logging
- `anyhow`, `thiserror` for errors
- `nix` for PTY, low-level syscalls (carefully)
- `zbus` or `dbus` if you use systemd D-Bus APIs (or call out to a privileged
  helper)
- `pcap` or a wrapper if you need packet capture
- `open-telemetry` / `prometheus` exporter for metrics

## Integration with `systemd-nspawn`

- Use `systemd-nspawn` templates to build minimal images.
- The `Container Manager` uses systemd machinery to create ephemeral containers
from images and attach them to sessions.
<!-- TODO: more research on this -->
