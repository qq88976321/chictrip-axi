# Usage

```
chictrip-axi [COMMAND] [OPTIONS]
```

Fifteen commands over the chicTrip API. Reads work with no setup; the
trip commands need a member token copied out of the browser. Sample
values below are romanized, the live API answers in zh-TW.

## Home view

A bare invocation prints live content, not help text (AXI: content
first):

```
$ chictrip-axi
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

| Line | Meaning |
|------|---------|
| `bin` | The path this binary runs from, so an agent can quote it in later calls |
| `description` | One sentence of what this AXI does |
| `auth` | `member` when a token is configured, `guest` otherwise |
| `trips` / `tours` | Your five most recently updated trips, or the five most popular expert itineraries for a guest |
| `commands` | Every command, one clause each |
| `help` | The commands worth running next |

If the one live call fails, the identity and the command index still
print and the exit code is `1`: an agent that cannot reach the API
still needs to know what this tool does.

## Commands

### Reading, no setup needed

| Command | What it answers |
|---------|-----------------|
| `poi search <keyword> [--near LAT,LNG] [--limit N]` | Places and their ids |
| `poi view <poi-id> [--full]` | Address, hours, rating, description, media and ticket counts |
| `tour list [--curated] [--page N] [--limit N]` | Popular expert itineraries, or the editor picks |
| `tour view <tour-id> [--day N] [--full]` | An expert itinerary day by day |
| `location search <keyword> [--limit N]` | Destination keys (country,city,area) for `trip create` |

### Your account

| Command | What it does |
|---------|--------------|
| `auth set (--from-json FILE\|- \| --access-token T --member-id M)` | Stores the session |
| `auth status` | Names the effective token source and verifies it |
| `auth clear` | Forgets the stored session |
| `trip list [--limit N]` | Your trips, newest first |
| `trip create --name N --start D --end D --location K... [--traffic M] [--duplicate]` | An empty trip filed under one or more destination keys |
| `trip view <trip-id> [--day N]` | Stops day by day, with the `tsd_id` |
| `trip add <trip-id> --day N --poi ID... [--position last\|best] [--allow-duplicate]` | Appends POIs to a day |
| `trip remove <trip-id> --stop TSD-ID...` | Removes stops |
| `trip delete <trip-id>` | Deletes the trip |
| `tour copy <tour-id>` | Copies an expert itinerary into your trips |

Every command keeps a concise `--help` with its flags, defaults, and
two or three examples.

## Signing in

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

The three values go to `$XDG_CONFIG_HOME/chictrip-axi/auth.json`
(falling back to `~/.config/chictrip-axi/auth.json`) with mode `0600`.
When the access token expires the client refreshes it and replays the
request; a token passed with `--token` or `CHICTRIP_AXI_TOKEN` is never
rewritten.

Token precedence: `--token`, then `CHICTRIP_AXI_TOKEN`, then the auth
file, then chicTrip's public guest token. Trip commands refuse the
guest token locally, before any request, because chicTrip would answer
with the demo account's data instead.

## Plan a trip and write it

```
chictrip-axi location search Kyoto
chictrip-axi trip create --name "Kyoto weekend" --start 2026-11-07 --end 2026-11-08 --location 7,9,45
chictrip-axi tour list --limit 5
chictrip-axi tour view <tour-id> --day 1
chictrip-axi tour copy <tour-id>                        # or start from an expert itinerary
chictrip-axi trip view <trip-id>
chictrip-axi poi search "Kamakura" --limit 5
chictrip-axi trip add <trip-id> --day 2 --poi <poi-id> --poi <poi-id>
chictrip-axi trip remove <trip-id> --stop <tsd-id>
chictrip-axi trip delete <trip-id>
```

Mutations are idempotent where they can be: `trip add` skips a POI the
day already contains, `trip create` returns the existing trip when the
name and dates match, and `trip delete` succeeds on a trip that is
already gone. All three exit `0`.

## Output

stdout is [TOON](https://toonformat.dev/). A list prints its `count`,
then the table, then the next commands:

```
$ chictrip-axi poi search "Senso-ji" --limit 2
count: 2 of 7
pois[2]{id,name,category,rating,city,area}:
  "8a48a94c-...",Senso-ji Kaminarimon,enterTainment,4.5,Tokyo,Taito
  "200e3d31-...",Senso-ji,enterTainment,4.5,Tokyo,Taito
help[2]: ...
```

- `count: 2 of 7` says how many rows `--limit` trimmed; an empty result
  is `count: 0` plus `pois[0]:`, never silence.
- `--json` prints the same document as one line of compact JSON.
- `--fields a,b,c` restricts a table or a detail to those fields in that
  order, and reveals the extra columns each command documents; an
  unknown name is a usage error that lists the valid ones.
- `--full` expands truncated text and the tables a detail summarises.
- Long text is cut at 500 characters with `... (truncated, N chars
  total)` and a hint naming the `--full` command.

Only the document goes to stdout. Nothing is written to stderr in
normal operation.

## Help and version

```
chictrip-axi --help            # concise reference for this level
chictrip-axi trip add --help   # flags, defaults, and examples for one command
chictrip-axi --version         # bare version, e.g. chictrip-axi 0.0.1
```

`-V` is a synonym for `--version`. Both print the bare version and exit
`0` before anything else is loaded, so an agent can check what it is
talking to cheaply.

## Exit codes and errors

| Exit code | Meaning |
|-----------|---------|
| `0` | Success, including no-ops |
| `1` | The command ran but could not complete |
| `2` | Usage error, decided before any network call |

Errors are structured on stdout in the same format as normal output,
with a stable code and the command that fixes the problem:

```
$ chictrip-axi poi search "Senso-ji" --bogus
error: usage
message: "unexpected argument '--bogus' found"
flags[7]: "--fields","--help","--json","--limit","--near","--timeout","--token"
help[1]: "Run `chictrip-axi poi search --help` for the flags and defaults and examples"
$ echo $?
2
```

The valid flags are inlined so the correction takes one turn instead of
a second call to `--help`.

| `error` | When |
|---------|------|
| `usage` | argv rejected, bad date, bad `--near`, unknown `--fields` name |
| `auth_required` | a trip command with no member token, or chicTrip refusing a guest |
| `auth_invalid` | the stored token is expired and could not be refreshed |
| `not_found` | unknown trip, POI, day, or stop |
| `forbidden` | someone else's trip |
| `conflict` | the trip changed under us twice in a row |
| `api_error` | anything else chicTrip refused; `message` is its own sentence |
| `network` | connect, DNS, TLS, or timeout; names the host and the timeout |
| `internal` | chicTrip answered something we cannot parse |

An API error also carries chicTrip's `request_id`. Raw upstream bodies
and stack traces never reach stdout, and no command prompts
interactively.

## Environment

| Variable | Meaning |
|----------|---------|
| `CHICTRIP_AXI_TOKEN` | Use this member token (same as `--token`, and never refreshed) |
| `CHICTRIP_AXI_AUTH_FILE` | Where the session is stored |
| `CHICTRIP_AXI_BASE_URL` | Point the client somewhere else; defaults to `https://api.chictrip.com.tw/` |

Global flags accepted by every command, and never reported as unknown:
`--json`, `--fields`, `--timeout SECS` (default 30), `--token JWT`,
`--help`, and `-V`/`--version` at the top level.
