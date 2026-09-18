---
name: chictrip-axi
description: Plan trips on chicTrip (chictrip.com.tw) from the shell with the chictrip-axi CLI. Use when asked to search places or destinations, read expert itineraries, or create, fill, inspect, and refine trips in a chicTrip account: stop times and stays, notes, how to travel between stops, the order of a day, flights, and places chicTrip does not list.
---

# chictrip-axi

Agent-first CLI for chicTrip: search places, read expert itineraries, build and edit trips in your account

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
- Ids are UUIDs; pass them back verbatim. Mutations are idempotent: `trip create` with the same name and dates returns the existing trip, `trip add` skips POIs already in the day, `trip remove` skips a stop that is not there, `trip delete` on a missing trip succeeds, and `trip edit`, `trip note`, `trip leg`, `trip traffic`, and `trip move` write nothing when the trip already matches the request (they say `no-op`). The exception is `poi create`: a private place cannot be found again, so every run files another one and chicTrip cannot delete it.
- Nothing prompts; every command is safe to run unattended.

## Auth

Read commands work with the built-in guest token. `trip *` (except `trip preview`), `tour copy`, and `poi create` need a member token. chicTrip has no password login, so log in at https://www.chictrip.com.tw/ and run this in the browser console:

```js
copy(JSON.stringify({accessToken:localStorage.accessToken,refreshToken:localStorage.refreshToken,memberId:localStorage.memberId}))
```

Then run `chictrip-axi auth set --from-json -` and paste. `chictrip-axi auth status` says which token is active and whether it works.

## Commands

- `chictrip-axi auth set` - Store a member token copied from the browser
- `chictrip-axi auth status` - Show which token is in use and whether it works
- `chictrip-axi auth clear` - Forget the stored member token
- `chictrip-axi trip list` - My trips (newest first)
- `chictrip-axi trip create --name --start --end` - Create an empty trip
- `chictrip-axi trip view <trip-id>` - Stops of a trip day by day (--full for notes and legs)
- `chictrip-axi trip preview <trip-id>` - Stops of any trip by id (no login needed)
- `chictrip-axi trip add <trip-id> --day N --poi ID...` - Add POIs to a day (skips duplicates)
- `chictrip-axi trip edit <trip-id> --stop ID` - Change a stop's stay or times or category or name
- `chictrip-axi trip note <trip-id> [--stop ID]` - Read or set the trip note or a stop's
- `chictrip-axi trip leg <trip-id> --stop ID` - List or set how a stop is reached
- `chictrip-axi trip traffic <trip-id> --day N --mode M` - Set a day's default travel mode
- `chictrip-axi trip move <trip-id> --stop ID` - Move a stop within or across days
- `chictrip-axi trip remove <trip-id> --stop ID...` - Remove stops
- `chictrip-axi trip delete <trip-id>` - Delete a whole trip
- `chictrip-axi tour list` - Popular expert itineraries (--curated for editor picks)
- `chictrip-axi tour view <tour-id>` - An expert itinerary day by day
- `chictrip-axi tour copy <tour-id>` - Copy an expert itinerary into my trips
- `chictrip-axi poi search <keyword>` - Find places and their ids
- `chictrip-axi poi view <poi-id>` - Address and hours and rating and description of a place
- `chictrip-axi poi create --name --at` - File a private place chicTrip does not list (not idempotent)
- `chictrip-axi location search <keyword>` - Destination keys for trip create
- `chictrip-axi setup skill` - Write the agent skill file (--check verifies it)
- `chictrip-axi setup hooks` - Install the Claude Code SessionStart hook on request

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

Refine a day:

1. `chictrip-axi trip view <trip-id> --day 1 --full` shows each stop's `tsd_id`, arrival, stay, category, note, and leg
2. `chictrip-axi trip edit <trip-id> --stop <tsd-id> --arrive 10:30 --stay 90` pins the arrival and the stay; `--arrive auto` lets chicTrip compute it again; `--category takeOff` or `--category landing` turns a stop into a flight row
3. `chictrip-axi trip note <trip-id> --stop <tsd-id> --set "book ahead"` writes a note on a stop; without `--stop` it is the trip note (chicTrip has no per-day note)
4. `chictrip-axi trip leg <trip-id> --stop <tsd-id>` lists how to reach that stop; `--route <route_id>` picks one, `--custom 25 --note "taxi"` or `--flight 180 --note "BR198 TPE-NRT"` writes a free-form leg. A leg belongs to the stop you arrive at, so a day's first stop has none; `chictrip-axi trip traffic <trip-id> --day 1 --mode transit` sets the whole day's default
5. `chictrip-axi trip move <trip-id> --stop <tsd-id> --position first` reorders the day; `--day 2 --position last` moves the stop into another day, keeping its id and its note
6. `chictrip-axi poi create --name "Our ryokan" --at 35.0116,135.7681 --category moon` files a place chicTrip does not list; use its id with `trip add`
