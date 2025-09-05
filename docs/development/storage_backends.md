# Storage Backend Selection in Miel

This document demonstrates how to use the new storage backend selection feature
in Miel.

## Overview

You can now choose between two storage backends:

- **Database** (SQLite): Default option, stores data in a SQLite database
- **FileSystem**: Stores data as files in the filesystem

## Configuration Methods

### 1. Configuration File

Add the `storage_backend` field to your TOML configuration file:

```toml
bind_address = "127.0.0.1"
storage_path = "/var/lib/miel"
storage_backend = "database"  # Options: "database" or "filesystem"
web_ui_enabled = true
web_ui_port = 3000
max_sessions = 100
session_timeout_secs = 3600
```

### 2. Command Line Interface

Use the `--storage-backend` flag when running the application:

```bash
# Use database storage (default)
./miel config.toml --storage-backend database

# Use filesystem storage
./miel config.toml --storage-backend filesystem

# You can also override other settings
./miel config.toml --storage-backend filesystem --storage-path /custom/path
```

## Storage Backend Details

### Database Backend

- **Format**: SQLite database
- **Location**: `{storage_path}/miel.sqlite3`
- **Pros**:
  - Structured queries
  - Better performance for large datasets
  - ACID compliance
- **Cons**:
  - Less human-readable
  - Requires SQLite tools for inspection

### Filesystem Backend

- **Format**: Human-readable files
- **Structure**:

  ```txt
  {storage_path}/file_storage/
  ├── sessions/           # Session metadata (.session files)
  ├── interactions/       # Raw interaction data (.bin files)
  └── artifacts/          # Capture artifacts organized by session ID
      └── {session-id}/
          ├── tcp_capture.bin
          ├── stdio_capture.csv
          └── meta.txt
  ```

- **Pros**:
  - Human-readable
  - Easy to inspect and debug
  - Simple backup/restore
- **Cons**:
  - Slower for large datasets
  - More disk space usage

## Examples

### Example 1: Database Storage with Custom Path

```bash
./miel config.toml --storage-backend database --storage-path /opt/miel-data
```

This creates: `/opt/miel-data/miel.sqlite3`

### Example 2: FileSystem Storage

```bash
./miel config.toml --storage-backend filesystem --storage-path /opt/miel-data
```

This creates: `/opt/miel-data/file_storage/` with subdirectories

### Example 3: Mixed Configuration

```toml
# config.toml
bind_address = "0.0.0.0"
storage_path = "/var/log/honeypot"
storage_backend = "filesystem"
web_ui_enabled = true
web_ui_port = 8080
max_sessions = 50
```

```bash
# Override to use database instead
./miel config.toml --storage-backend database
```

## Migration Between Backends

Currently, there's no automatic migration between storage backends. If you need
to switch:

1. Stop the application
2. Backup your current data
3. Change the storage backend configuration
4. Restart the application (new backend will be initialized)

## Environment Variable Support

The storage path can also be controlled via environment variables:

```bash
export MIEL_STORAGE_PATH=/custom/storage/path
./miel config.toml --storage-backend filesystem
```

This will use `/custom/storage/path/file_storage/` for filesystem backend or
`/custom/storage/path/miel.sqlite3` for database backend.
