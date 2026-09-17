# chictrip-axi

An agent-first command-line client for the [chicTrip](https://chictrip.com.tw/)
travel API, built to the [AXI](https://axi.md/) (Agent eXperience
Interface) principles: output an agent can read cheaply, errors it can
fix in one turn, and no prompt it can get stuck on. Written in Rust,
shipped as a single static binary.

Docs site: <https://qq88976321.github.io/chictrip-axi/>.

## Status

Fourteen commands over the chicTrip API: search places, read expert
itineraries, and build trips in your own account. Reads work with no
setup at all; the trip commands need a token you copy out of the
browser once. Every command was exercised against the live API on
2026-09-18, the trip commands with a real member account.

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
sh install.sh --to ~/bin --version v0.0.1
```

Uninstall by deleting the binary (`rm ~/.local/bin/chictrip-axi`).

### From source

```
cargo install --path .
```

Requires Rust 1.85+ (edition 2024).

## Usage

Sample values below are romanized; the live API answers in zh-TW.

### Read without signing in

```
chictrip-axi                                  # home view: identity, live content, every command
chictrip-axi poi search "Senso-ji"            # places and their ids
chictrip-axi poi view <poi-id>                # address, hours, rating, description
chictrip-axi tour list                        # popular expert itineraries
chictrip-axi tour view <tour-id> --day 1      # one day of an itinerary
```

A bare invocation prints live content rather than help text:

```
bin: ~/.local/bin/chictrip-axi
description: Agent-first CLI for chicTrip: search places, read expert itineraries, build trips in your account
auth: guest
tours[5]{id,name,destination,expert,likes}:
  "daebf5f2-...",Tokyo 7 days 6 nights,Japan,Mamo,4014
  ...
commands[14]{command,summary}:
  auth set,Store a member token copied from the browser
  ...
help[2]: ...
```

### Sign in

chicTrip has no password login, so the token is copied out of a browser
session. Log in at <https://www.chictrip.com.tw/>, open the developer
console, and run:

```js
copy(JSON.stringify({accessToken:localStorage.accessToken,refreshToken:localStorage.refreshToken,memberId:localStorage.memberId}))
```

Then paste it into:

```
chictrip-axi auth set --from-json -
chictrip-axi auth status
```

The three values are stored at
`$XDG_CONFIG_HOME/chictrip-axi/auth.json` (falling back to
`~/.config/chictrip-axi/auth.json`) with mode `0600`, and refreshed
automatically when the access token expires. `chictrip-axi auth clear`
forgets them.

### Plan a trip and write it

```
chictrip-axi trip list
chictrip-axi trip create --name "Kamakura weekend" --start 2026-11-07 --end 2026-11-08 --location 7,8,44
chictrip-axi tour copy <tour-id>                       # or start from an expert itinerary
chictrip-axi trip view <trip-id>                       # stops day by day, with the tsd_id
chictrip-axi poi search "Kamakura" --limit 5           # find ids to add
chictrip-axi trip add <trip-id> --day 2 --poi <poi-id> --poi <poi-id>
chictrip-axi trip remove <trip-id> --stop <tsd-id>     # undo one stop
chictrip-axi trip delete <trip-id>
```

`trip add` skips a POI the day already contains, `trip create` returns
the existing trip when the name and dates match, and `trip delete`
succeeds on a trip that is already gone, so a retried turn cannot make
a mess. Nothing prompts; every value is a flag.

### Output

stdout is [TOON](https://toonformat.dev/): a list prints its `count`,
then a table, then the commands worth running next.

```
$ chictrip-axi poi search "Senso-ji" --limit 2
count: 2 of 7
pois[2]{id,name,category,rating,city,area}:
  "8a48a94c-...",Senso-ji Kaminarimon,enterTainment,4.5,Tokyo,Taito
  "200e3d31-...",Senso-ji,enterTainment,4.5,Tokyo,Taito
help[2]: ...
```

- `--json` prints the same document as one line of compact JSON.
- `--fields a,b,c` restricts the table (or the detail) to those fields,
  in that order, and reveals the extras each command documents in its
  `--help`.
- `--full` expands the truncated text and the tables a detail view
  summarises.
- `--limit`, `--page`, `--day`, `--near` narrow what is fetched.

### Exit codes and errors

| Exit code | Meaning |
|-----------|---------|
| `0` | Success, including no-ops |
| `1` | The command ran but could not complete |
| `2` | Usage error, decided before any network call |

Errors are structured on stdout in the same format as normal output,
with a stable `error` code and the command that fixes it:

```
$ chictrip-axi poi search "Senso-ji" --bogus
error: usage
message: "unexpected argument '--bogus' found"
flags[7]: "--fields","--help","--json","--limit","--near","--timeout","--token"
help[1]: "Run `chictrip-axi poi search --help` for the flags and defaults and examples"
```

| `error` | When |
|---------|------|
| `usage` | argv rejected, bad date, bad `--near`, unknown `--fields` name |
| `auth_required` | a trip command with no member token, or chicTrip refusing a guest |
| `auth_invalid` | the stored token is expired and could not be refreshed |
| `not_found` | unknown trip, POI, day, or stop |
| `forbidden` | someone else's trip |
| `conflict` | the trip changed under us twice in a row |
| `api_error` | anything else chicTrip refused; `message` is its own sentence |
| `network` | connect, DNS, TLS, or timeout |
| `internal` | chicTrip answered something we cannot parse |

### Environment

| Variable | Meaning |
|----------|---------|
| `CHICTRIP_AXI_TOKEN` | Use this member token (same as `--token`, and never refreshed) |
| `CHICTRIP_AXI_AUTH_FILE` | Where the session is stored; defaults to `$XDG_CONFIG_HOME/chictrip-axi/auth.json` |
| `CHICTRIP_AXI_BASE_URL` | Point the client somewhere else; defaults to `https://api.chictrip.com.tw/` |

Token precedence is `--token`, then `CHICTRIP_AXI_TOKEN`, then the auth
file, then chicTrip's public guest token. The full contract lives on the
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

Releases are cut with `just release patch|minor|major` (cargo-release +
git-cliff); pushing the tag runs the release workflow. See
[docs/releasing.md](docs/releasing.md).

The development contract, for agents and humans alike, is
[CLAUDE.md](CLAUDE.md).

## License

MIT
