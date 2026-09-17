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

# cargo-release sets NEW_VERSION and DRY_RUN; on a dry run the
# changelog is left untouched so the working tree stays clean. Needs
# git-cliff on PATH.
# cargo-release pre-release hook: gate, then regenerate CHANGELOG.md.
release-hook:
    just gate
    if [ "$DRY_RUN" != "true" ]; then git-cliff --tag "v${NEW_VERSION}" -o CHANGELOG.md; fi

# First release: `just release 0.1.0` (explicit version -> tag v0.1.0,
# no patch bump); afterwards `just release patch|minor|major`.
# Bump version and tag vX.Y.Z locally (no push).
release level="patch":
    cargo release {{level}} --execute
