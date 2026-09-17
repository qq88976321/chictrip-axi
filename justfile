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

# Lint the shell scripts we ship (install.sh is executed by users
# straight off a release asset, so it has to be POSIX sh clean).
# Enforced in CI; kept out of `gate` so the gate does not silently
# skip a step on a machine without shellcheck.
lint-sh:
    shellcheck -s sh install.sh scripts/test-install.sh

# End-to-end round trip for install.sh against a fake release tree
# served over file://. Builds the musl asset first, so it needs
# `rustup target add x86_64-unknown-linux-musl`. This is the only way
# to exercise the install path without cutting a real release.
test-install:
    sh scripts/test-install.sh

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
