---
name: chictrip-axi
description: Plan trips on chicTrip (chictrip.com.tw) from the shell with the chictrip-axi CLI. Use when asked to search places or destinations, read expert itineraries, or create, fill, and inspect trips in a chicTrip account.
---

# chictrip-axi

{{description}}

## Install and verify

```sh
curl -fsSL https://github.com/qq88976321/chictrip-axi/releases/latest/download/install.sh | sh
chictrip-axi --version
```

The installer puts the binary in `~/.local/bin`. Run `chictrip-axi` with no arguments for the home view (identity, live content, the command index) and `chictrip-axi <command> --help` for flags, defaults, and examples.

## Output contract

- stdout is TOON. `--json` prints the same document as one line of JSON; `--fields a,b,c` keeps only those fields, in that order.
- Lists open with `count:` and print a compact default schema; `count: 20 of 35` means `--limit` trimmed rows. `--fields` reveals the extra columns each command documents in its `--help`; `--full` expands a detail view's truncated text and summarised tables.
- An empty result prints `count: 0` and an empty table like `pois[0]:`, never silence, and still exits 0.
- Errors are a document on stdout, not stderr: `error: <code>`, `message`, `help[]` with the command that fixes it, and `flags[]` on a usage error. Codes are stable: usage, auth_required, auth_invalid, not_found, forbidden, conflict, api_error, network, internal.
- Exit codes: 0 success (no-ops included), 1 error, 2 usage error.
- Ids are UUIDs; pass them back verbatim. Mutations are idempotent: `trip create` with the same name and dates returns the existing trip, `trip add` skips POIs already in the day, `trip remove` skips a stop that is not there, and `trip delete` on a missing trip succeeds.
- Nothing prompts; every command is safe to run unattended.

## Auth

Read commands work with the built-in guest token. `trip *` (except `trip preview`) and `tour copy` need a member token. chicTrip has no password login, so log in at https://www.chictrip.com.tw/ and run this in the browser console:

```js
copy(JSON.stringify({accessToken:localStorage.accessToken,refreshToken:localStorage.refreshToken,memberId:localStorage.memberId}))
```

Then run `chictrip-axi auth set --from-json -` and paste. `chictrip-axi auth status` says which token is active and whether it works.

## Commands

{{commands}}

## Recipes

Plan a trip:

1. `chictrip-axi location search Tokyo` gives the destination key (like `7,7,0`)
2. `chictrip-axi trip create --name "Tokyo temples" --start 2026-10-01 --end 2026-10-03 --location 7,7,0` gives the trip id
3. `chictrip-axi poi search "Senso-ji"` gives POI ids
4. `chictrip-axi trip add <trip-id> --day 1 --poi <poi-id> --poi <poi-id>`
5. `chictrip-axi trip view <trip-id>` confirms the plan; its `tsd_id` column feeds `trip remove`

Reference an expert itinerary or somebody else's trip:

- `chictrip-axi tour list`, then `chictrip-axi tour view <tour-id> --day 1`; `chictrip-axi tour copy <tour-id>` copies it into my trips (chicTrip allows several copies, so this one is not idempotent)
- `chictrip-axi trip preview <trip-id>` reads any trip by id, shared or not, without owning it
