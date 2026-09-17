# Install

## Quick install

```
curl -fsSL https://github.com/qq88976321/chictrip-axi/releases/latest/download/install.sh | sh
```

The script downloads the prebuilt binary for your platform, verifies it
against the sha256 published with the release, and installs it into
`~/.local/bin`. Nothing needs `sudo`. If that directory is not on your
`PATH`, the script tells you what to add to your shell profile.

!!! tip "Read before you run"

    Piping a script into a shell is worth a look first:

    ```
    curl -fsSL https://github.com/qq88976321/chictrip-axi/releases/latest/download/install.sh -o install.sh
    less install.sh
    sh install.sh
    ```

    The installer ships as an asset of each release, so the copy you
    fetch is the one written against that release's assets.

### Options

```
install.sh [--version <tag>] [--to <dir>]
```

| Option | Environment | Effect |
|--------|-------------|--------|
| `--version <tag>` | `CHICTRIP_AXI_VERSION` | Install that release instead of the latest, e.g. `v0.1.0` |
| `--to <dir>` | `CHICTRIP_AXI_INSTALL_DIR` | Install into `<dir>` instead of `~/.local/bin` |

Flags win over the environment. To pass a flag through the one-liner,
give the shell `-s --`:

```
curl -fsSL https://github.com/qq88976321/chictrip-axi/releases/latest/download/install.sh | sh -s -- --to ~/bin
```

## Supported platforms

| Platform | Release asset |
|----------|---------------|
| Linux x86_64 | `chictrip-axi-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 | `chictrip-axi-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `chictrip-axi-x86_64-apple-darwin.tar.gz` |
| macOS Apple silicon | `chictrip-axi-aarch64-apple-darwin.tar.gz` |

The Linux binaries are statically linked against musl, so one build per
architecture runs on every distribution whatever glibc version it ships -
Alpine included. Windows has no prebuilt binary; build from source there.

Every asset is published with a `chictrip-axi-<target>.sha256` sidecar,
so a manual download can be checked the same way the installer does:

```
sha256sum -c chictrip-axi-x86_64-unknown-linux-musl.sha256
```

## From source

chictrip-axi is a standard Cargo project:

```
cargo install --path .
```

### Requirements

- Rust 1.85 or newer (the crate uses edition 2024).
- No runtime dependencies: the binary is self-contained.

To build without installing:

```
cargo build --release
# the binary is at target/release/chictrip-axi
```

Copy `target/release/chictrip-axi` somewhere on your `PATH`.

## Verify

```
chictrip-axi --version
chictrip-axi --help
```

## Uninstall

Delete the binary:

```
rm ~/.local/bin/chictrip-axi
```

If you installed with Cargo, use `cargo uninstall chictrip-axi` instead.
