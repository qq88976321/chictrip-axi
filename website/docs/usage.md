# Usage

```
chictrip-axi [--help] [--version]
```

This release is the infrastructure milestone: the binary, the installer,
the release pipeline, and this site. The chicTrip API commands land in
the next milestone, and this page grows with them.

## Home view

A bare invocation prints live content, not help text (AXI: content
first):

```
$ chictrip-axi
bin: /home/you/.local/bin/chictrip-axi
description: Agent-first CLI for the chicTrip travel API
commands: 0 (API commands land in the next milestone)
next: chictrip-axi --help
```

| Line | Meaning |
|------|---------|
| `bin` | The path this binary runs from, so an agent can quote it in later calls |
| `description` | One sentence of what this AXI does |
| `commands` | How many subcommands exist right now |
| `next` | The command worth running next |

## Help and version

```
chictrip-axi --help      # concise reference for this level
chictrip-axi --version   # bare version, e.g. chictrip-axi 0.1.0
```

`-V` is a synonym for `--version`. Both print the bare version and exit
`0` before anything else is loaded, so an agent can check what it is
talking to cheaply.

## Exit codes and errors

| Exit code | Meaning |
|-----------|---------|
| `0` | Success, including no-ops |
| `1` | The command ran but could not complete |
| `2` | Usage error: unknown flag, missing value |

An unknown flag is rejected before anything else runs:

```
$ chictrip-axi --bogus
error: unexpected argument '--bogus' found

Usage: chictrip-axi

For more information, try '--help'.
$ echo $?
2
```

No command prompts interactively; everything is flag-completable, so it
is safe to run from an agent or a script.

## The AXI contract

Every command added to this CLI follows the [AXI](https://axi.md/)
principles:

- TOON on stdout, with `--json` as the escape hatch; default list
  schemas of three or four fields, pre-computed totals (`count: 30 of
  847 total`), truncation with size hints, and a definitive `0 results`
  line instead of silence.
- Structured errors on stdout with stable codes, validated before any
  network call, never a raw upstream body or a stack trace.
- Idempotent mutations, no interactive prompts, and contextual next-step
  suggestions after each output.
