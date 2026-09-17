# chictrip-axi

An agent-first command-line client for the [chicTrip](https://chictrip.com.tw/)
travel API, built to the [AXI](https://axi.md/) (Agent eXperience
Interface) principles: output an agent can read cheaply, errors it can
fix in one turn, and no prompt it can get stuck on. Written in Rust,
shipped as a single static binary.

Docs site: <https://qq88976321.github.io/chictrip-axi/>.

## Status

Infrastructure milestone: the binary, the installer, the release
pipeline, and the docs site are in place. The chicTrip API commands
land in the next milestone; today the binary prints its home view,
`--help`, and `--version`.

Personal tool, built with heavy AI assistance (Claude Code). I review
what ships, but it comes with no warranty and no support commitment:
issues and PRs are welcome and may still go unanswered. An independent
project, not affiliated with or endorsed by chicTrip.

## Install

```
curl -fsSL https://github.com/qq88976321/chictrip-axi/releases/latest/download/install.sh | sh
```

This downloads the prebuilt binary for your platform, checks it against
the release's sha256, and installs it into `~/.local/bin` - no `sudo`
anywhere.

| Platform | Release asset |
|----------|---------------|
| Linux x86_64 | `chictrip-axi-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 | `chictrip-axi-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `chictrip-axi-x86_64-apple-darwin.tar.gz` |
| macOS Apple silicon | `chictrip-axi-aarch64-apple-darwin.tar.gz` |

The Linux builds are statically linked against musl, so they do not care
which glibc your distribution ships. Windows is not covered - build from
source there.

Rather read the script first, install somewhere else, or pin a version:

```
curl -fsSL https://github.com/qq88976321/chictrip-axi/releases/latest/download/install.sh -o install.sh
less install.sh
sh install.sh --to ~/bin --version v0.1.0
```

Uninstall by deleting the binary (`rm ~/.local/bin/chictrip-axi`).

### From source

```
cargo install --path .
```

Requires Rust 1.85+ (edition 2024).

## Usage

```
chictrip-axi             # home view: what this binary is and what to run next
chictrip-axi --help      # concise reference
chictrip-axi --version   # bare version, e.g. chictrip-axi 0.1.0
```

A bare invocation prints live content rather than help text:

```
bin: /home/you/.local/bin/chictrip-axi
description: Agent-first CLI for the chicTrip travel API
commands: 0 (API commands land in the next milestone)
next: chictrip-axi --help
```

Exit codes follow the AXI contract: `0` success (including no-ops),
`1` error, `2` usage error (an unknown flag is rejected before anything
runs). Nothing prompts interactively. The contract every future command
follows is spelled out on the
[docs site](https://qq88976321.github.io/chictrip-axi/usage/).

## Development

```
just gate          # fmt --check, clippy -D warnings, test, release build
just test          # unit tests
just build         # debug build
just run -- --help
just lint-sh       # shellcheck the shipped shell scripts
just test-install  # offline install.sh round trip (needs the musl target)
just site-build    # docs site (Zensical via uvx)
```

Releases are cut with `just release` (cargo-release + git-cliff); the
first release is `just release 0.1.0`. Pushing the tag runs the release
workflow; see [docs/releasing.md](docs/releasing.md).

The development contract, for agents and humans alike, is
[CLAUDE.md](CLAUDE.md).

## License

MIT
