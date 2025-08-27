# Configuration parser

## Crates

- `seder`: Framework allowing to use `Serialize` and `Deserialize` traits on custom structs
- `toml`: Uses the `seder` traits to `Serialize` or `Deserialize` to/from TOML

In our case implementing only `Deserialize` on the struct is enough as we would only read from the configuration file

```rust
#[derive(Debug, Deserialize)]
struct Config {
// ...
}
```

- `clap`: Command Line Arguments Parser used to parse the arguments passed in the command line

