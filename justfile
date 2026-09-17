# Entry points for chictrip-axi development and release.

# List available recipes.
default:
    @just --list

# Debug build.
build:
    cargo build

# Remove with `cargo uninstall chictrip-axi`.
# Install the optimized `chictrip-axi` binary to ~/.cargo/bin.
install:
    cargo install --path .

# Much quicker than `install` after code changes, since it skips the
# release profile's whole-program LTO; use `install` only when you
# actually want the optimized binary.
# Fast local install (dev profile, no LTO) of `chictrip-axi` onto PATH.
install-dev:
    cargo install --path . --debug

# Run unit tests.
test:
    cargo test

# Full quality gate; run before every commit.
gate:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo build --release

# Run the binary; pass extra args, e.g. `just run -- --help`.
run *args:
    cargo run -- {{args}}
