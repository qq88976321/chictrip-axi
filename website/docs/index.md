# chictrip-axi

An agent-first command-line client for the [chicTrip](https://chictrip.com.tw/)
travel API, built to the [AXI](https://axi.md/) (Agent eXperience
Interface) principles: output an agent can read cheaply, errors it can
fix in one turn, and no prompt it can get stuck on. Written in Rust and
shipped as a single static binary.

## The contract

Every command this CLI grows follows the AXI design:

- **Token-efficient output** - TOON on stdout (about 40% fewer tokens
  than the equivalent JSON), minimal default schemas, pre-computed totals,
  truncation with size hints, definitive empty states.
- **Self-correcting errors** - structured errors with stable exit codes
  (`0` success, `1` error, `2` usage), never a stack trace, never an
  interactive prompt.
- **Content first** - a bare `chictrip-axi` shows live state, not help
  text, and points at the next useful command; every subcommand keeps a
  concise `--help`.
- **One static binary** - musl builds for Linux, native builds for
  macOS, installed with a checksum-verified one-liner and no `sudo`.

## Install

```
curl -fsSL https://github.com/qq88976321/chictrip-axi/releases/latest/download/install.sh | sh
```

A checksum-verified prebuilt binary for Linux and macOS, installed into
`~/.local/bin` without `sudo`. See [Install](install.md) for the flags,
the platform list, and building from source.

## At a glance

```
chictrip-axi                                  # home view: identity, live content, every command
chictrip-axi poi search "Senso-ji"            # places and their ids
chictrip-axi tour view <tour-id> --day 1      # an expert itinerary, one day
chictrip-axi auth set --from-json -           # paste the token copied from the browser
chictrip-axi trip add <trip-id> --day 2 --poi <poi-id>
chictrip-axi trip preview <trip-id>           # any trip by id, no sign-in
chictrip-axi setup hooks                      # home view at every agent session start
```

Reads work with no setup at all. See [Usage](usage.md) for the command
list, signing in, the output format, the exit codes, and the error
contract.

## Status

!!! note "Personal tool, built with heavy AI assistance (Claude Code)"

    Eighteen commands: search places, read expert itineraries, build
    trips in your own account, and wire the CLI into your agent's
    session. It comes with no warranty and no support commitment:
    issues and PRs are welcome and may still go unanswered.

    An independent project, not affiliated with or endorsed by chicTrip.

Source on [GitHub](https://github.com/qq88976321/chictrip-axi),
[MIT licensed](https://github.com/qq88976321/chictrip-axi/blob/master/LICENSE).
