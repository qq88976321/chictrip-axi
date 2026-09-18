# AXI interface design: plan a trip, write it into chicTrip (milestone 2)

Status: APPROVED by the user on 2026-09-18 (decisions D1-D4 below);
implemented and verified live the same day, see "Live-run corrections"
at the end for where the implementation departs from this text.
Source facts: [../api/protocol.md](../api/protocol.md) and
[../api/endpoints.md](../api/endpoints.md). Design rules: the `axi`
skill (https://axi.md/) and the repo CLAUDE.md. Sample values are
romanized to keep this file ASCII; the live API returns zh-TW text.

## Purpose

The CLI exists so an agent can turn a plan ("three days in Tokyo,
temples on day 1") into a chicTrip itinerary in the user's account:
find POI ids, create the trip, fill each day, verify, undo. Reading
public data (POI search, expert itineraries) supports that flow; the
member-only write path is the product. Mapping natural language to
commands is the agent's job; the CLI makes the surface discoverable
(home view, `help` hints, per-command `--help`) and cheap to drive
(batch adds, idempotent retries, one turn per correction).

## Scope

In: `auth set/status/clear`, `trip list/create/view/add/remove/delete`,
`tour list/view/copy`, `poi search/view`, `location search` (added
during implementation: `trip create` cannot work without a destination
key, see the corrections), the home view, and the shared
HTTP/output/error/auth layers. Added in milestone 3: `trip preview
<trip-id>` for other people's trips, `setup skill` with its `--check`
gate, and `setup hooks`. Added in milestone 4 (its own section below):
`--full` on `trip view`/`trip preview` for a stop's note, leg, and
category; exact insertion for `trip add` (`--position first`,
`--after`); `trip edit`, `trip note`, `trip leg`, `trip traffic`,
`trip move`; and `poi create` for a place chicTrip does not list. Out
(later milestones): rankings, nearby, comments, categories (designed in
git history at `2834aac`), day-level operations
(`TravelSchedule/DeleteDay`, `UpdateStartDate`, `SortDay`), trip-level
edits (`UpdateV3`: rename, dates, cover), collaboration, chicTrip's
best sort (`PreviewBestSortByDayV2`/`SaveBestSortByDayV2`: 4 to 40
stops per day and a save body that was not cracked), a local UUID check
on positional ids (an id chicTrip rejects is reported as chicTrip
reports it), and hooks for Codex (`~/.codex/hooks.json`) and OpenCode
(a managed plugin).

## Command tree

```
chictrip-axi                                   home view
chictrip-axi auth set (--access-token T --refresh-token R --member-id M | --from-json FILE|-)
chictrip-axi auth status
chictrip-axi auth clear
chictrip-axi trip list [--limit N]
chictrip-axi trip create --name NAME --start DATE --end DATE --location KEY... [--traffic MODE] [--duplicate]
chictrip-axi trip view <trip-id> [--day N] [--full]
chictrip-axi trip preview <trip-id> [--day N] [--full]
chictrip-axi trip add <trip-id> --day N --poi POI-ID... [--position last|best|first] [--after TSD-ID] [--allow-duplicate]
chictrip-axi trip edit <trip-id> --stop TSD-ID [--stay MIN] [--arrive HH:MM|auto] [--depart HH:MM|auto] [--category TYPE] [--name TEXT]
chictrip-axi trip note <trip-id> [--stop TSD-ID] [--set TEXT | --clear]
chictrip-axi trip leg <trip-id> --stop TSD-ID [--mode driving|transit|walking|scooter | --route ROUTE-ID | --custom MIN [--note TEXT] | --flight MIN [--note TEXT]]
chictrip-axi trip traffic <trip-id> --day N --mode custom|driving|transit|walking|scooter [--recompute]
chictrip-axi trip move <trip-id> --stop TSD-ID (--after TSD-ID | --position first|last) [--day N]
chictrip-axi trip remove <trip-id> --stop TSD-ID...
chictrip-axi trip delete <trip-id>
chictrip-axi tour list [--curated] [--page N] [--limit N]
chictrip-axi tour view <tour-id> [--day N] [--full]
chictrip-axi tour copy <tour-id>
chictrip-axi poi search <keyword> [--near LAT,LNG] [--limit N]
chictrip-axi poi view <poi-id> [--full]
chictrip-axi poi create --name TEXT --at LAT,LNG [--category TYPE] [--address TEXT] [--note TEXT]
chictrip-axi location search <keyword> [--limit N]
chictrip-axi setup skill [--check] [--out PATH]
chictrip-axi setup hooks [--user] [--remove]
```

Global flags, accepted by every command and never reported as unknown:
`--json`, `--fields A,B,C`, `--timeout SECS` (default 30), `--token JWT`,
`--help`, `-V/--version`. Environment: `CHICTRIP_AXI_TOKEN` (same as
`--token`), `CHICTRIP_AXI_AUTH_FILE` (default
`$XDG_CONFIG_HOME/chictrip-axi/auth.json`, falling back to
`~/.config/chictrip-axi/auth.json`), `CHICTRIP_AXI_BASE_URL` (default
`https://api.chictrip.com.tw/`; tests point it at a local fixture
server and never touch the network).

Nouns are singular; ids are positional; filters and inputs are flags.
Every list caps rows with `--limit` (defaults below) and says so in
`count`. Dates are accepted as `YYYY-MM-DD` or `YYYY/MM/DD` and sent
to the API as `YYYY/MM/DD`.

## Authentication (D2)

Token precedence: `--token` > `CHICTRIP_AXI_TOKEN` > `accessToken` in
the auth file > the embedded guest token (chicTrip's public anonymous
JWT, see protocol.md). `auth set` writes the auth file with mode 0600:

```json
{"accessToken": "...", "refreshToken": "...", "memberId": "..."}
```

which is exactly what the browser snippet in the README produces, so
`auth set --from-json -` accepts it on stdin.

- Commands under `trip`, except `trip preview`, plus `tour copy`, need a
  member token. When the effective token is the embedded guest, they
  fail locally with `auth_required` before any network call (the guest
  "member" would otherwise answer with demo data).
- On `apiStatus 003` with a member token from the auth file and a
  refresh token present, the client calls `POST Token/Refresh`, saves
  the three new values, and retries the request once. Tokens from
  `--token` or the environment are never refreshed. A failed refresh
  is `auth_invalid` with the `auth set` command in `help`.
- The embedded guest token is a named constant with a comment saying
  where it comes from and that a rotation means a patch release; it is
  read-only fallback, never persisted.

## Output contract

- stdout is TOON (spec v4.1, flat shapes only: objects, primitive
  arrays, tabular arrays of uniform flat objects). `--json` prints the
  same document as one line of compact JSON. Nothing else goes to
  stdout; progress and debug go to stderr.
- Keys are `snake_case`. Values are left to the encoder's quoting
  rules. Numbers stay numbers; coordinates are `lat` and `lng`.
- Lists: `count` first, then the table, then `help`. `page` and
  `next_page` appear when the API pages (omit `next_page` when none).
  When `--limit` trimmed rows: `count: 20 of 35`.
- Empty results are definitive: `count: 0`, `trips[0]:`, plus a `help`
  with the next move. Exit 0.
- Long text (`description`, `introduction`, `note`) is cut at 500
  chars with `... (truncated, N chars total)` and a `--full` hint;
  hints appear only when something was cut. Null fields are omitted.
- `help[N]` lines are complete commands with `<placeholders>` for
  runtime values, carrying forward disambiguating flags. Detail views
  that fully answer the question print no `help`.
- `--fields` restricts a table or detail to the named fields in that
  order; an unknown name is a usage error listing the valid fields.
- Mutations print what changed (ids, new update time) and a `help`
  with the verify and undo commands.

## Error contract

Errors are TOON on stdout, then a non-zero exit.

```
error: auth_required
message: trip commands need a chicTrip member token
help[2]:
  Run `chictrip-axi auth set --from-json -` and paste the JSON copied from the browser (see README)
  Run `chictrip-axi auth status` to check what is configured
```

| code            | exit | when                                                        |
|-----------------|------|-------------------------------------------------------------|
| `usage`         | 2    | clap rejects argv, bad `--near`/date/`--fields`; before any network call; `flags[N]` lists the command's valid flags inline |
| `auth_required` | 1    | member command with the guest token (local check), or apiStatus 002 `Reject Guest Member` |
| `auth_invalid`  | 1    | apiStatus 003 after the refresh attempt (or with no refresh token) |
| `not_found`     | 1    | HTTP 404, apiStatus 011, id-format messages, unknown day/stop/POI in a trip |
| `forbidden`     | 1    | apiStatus 006                                                |
| `conflict`      | 1    | apiStatus 004 still returned after the one automatic retry  |
| `api_error`     | 1    | any other non-001 status; `message` is the upstream sentence verbatim |
| `network`       | 1    | connect/DNS/TLS failure or timeout; names host and timeout  |
| `internal`      | 1    | response did not parse as the envelope                      |

`request_id` from the envelope is added on API errors. Stack traces
and raw bodies never appear. `--help` and `--version` succeed before
anything else loads.

## Commands in detail

### Home view (bare invocation)

One network call. With a member token: my five most recently updated
trips (`TravelSchedule/GetMyAndCollaboration`). With the guest token:
five popular expert tours (`ExpertTour/PopularRanking?page=1&pageSize=5`)
and an `auth: guest` line. If the call fails, identity and the command
index still print and the exit code is 1.

```
bin: ~/.local/bin/chictrip-axi
description: Agent-first CLI for chicTrip: search places, read expert itineraries, build and edit trips in your account
auth: member
trips[3]{id,name,start,end,days}:
  fd4db85c-...,Tokyo temples,2026/10/01,2026/10/03,3
  ...
commands[24]{command,summary}:
  auth set,Store a member token copied from the browser
  auth status,Show which token is in use and whether it works
  auth clear,Forget the stored member token
  trip list,My trips (newest first)
  "trip create --name --start --end",Create an empty trip
  "trip view <trip-id>","Stops of a trip day by day (--full for notes and legs)"
  "trip preview <trip-id>",Stops of any trip by id (no login needed)
  "trip add <trip-id> --day N --poi ID...",Add POIs to a day (skips duplicates)
  "trip edit <trip-id> --stop ID",Change a stop's stay or times or category or name
  "trip note <trip-id> [--stop ID]",Read or set the trip note or a stop's
  "trip leg <trip-id> --stop ID",List or set how a stop is reached
  "trip traffic <trip-id> --day N --mode M",Set a day's default travel mode
  "trip move <trip-id> --stop ID",Move a stop within or across days
  "trip remove <trip-id> --stop ID...",Remove stops
  "trip delete <trip-id>",Delete a whole trip
  tour list,"Popular expert itineraries (--curated for editor picks)"
  "tour view <tour-id>",An expert itinerary day by day
  "tour copy <tour-id>",Copy an expert itinerary into my trips
  poi search <keyword>,Find places and their ids
  "poi view <poi-id>",Address and hours and rating and description of a place
  "poi create --name --at",File a private place chicTrip does not list (not idempotent)
  location search <keyword>,Destination keys for trip create
  setup skill,"Write the agent skill file (--check verifies it)"
  setup hooks,Install the Claude Code SessionStart hook on request
help[2]:
  Run `chictrip-axi trip view <id>` to continue a trip above
  Run `chictrip-axi <command> --help` for flags, defaults, and examples
```

The command index is the static part the `setup skill` generator reads;
keep summaries to one clause.

### auth set

Writes the auth file. Input either the three flags or `--from-json
FILE` / `--from-json -` (stdin) with the browser JSON. Validates that
`accessToken` parses as a JWT and reports its expiry. No network.

```
auth:
  member_id: 6bea8a8c-...
  access_token_expires: 2026-09-19T04:12:00Z
  file: ~/.config/chictrip-axi/auth.json
help[1]:
  Run `chictrip-axi auth status` to verify the token against the API
```

### auth status

Reports the effective source (`flag`, `env`, `file`, `guest`), member
id and expiry when known, and verifies a member token with
`GET TravelScheduleUserLabel/Get` (guest tokens are rejected there, so
it doubles as the liveness probe). Exit 0 whether member or guest;
`valid: false` with the API's message when the probe fails.

```
auth:
  source: file
  member_id: 6bea8a8c-...
  access_token_expires: 2026-09-19T04:12:00Z
  valid: true
```

### auth clear

Deletes the auth file. Already absent: `auth: nothing stored (no-op)`,
exit 0.

### trip list

`GET TravelSchedule/GetMyAndCollaboration?updateTime=0&orderByColumn=updatetime&sort=desc`,
default `--limit 20`.

```
count: 3
trips[3]{id,name,start,end,days,permission,updated}:
  fd4db85c-...,Tokyo temples,2026/10/01,2026/10/03,3,Owner,2026-09-18
help[2]:
  Run `chictrip-axi trip view <id>` for the day-by-day plan
  Run `chictrip-axi trip create --name "<name>" --start <date> --end <date>` to start a new trip
```

`updated` is `updateTime` as `YYYY-MM-DD` UTC. `--fields` extras:
`traffic`, `update_time` (raw seconds), `collaborators` (count),
`owner` (memberId).

### trip create

Steps: (1) `GET TravelScheduleUserLabel/Get` for the default label id
(the system entry named "unlabeled" in zh-TW, else the first); (2)
unless `--duplicate`, `trip list` and, if a trip with the same name,
start, and end exists, return it with `note: already exists (no-op)`
and exit 0; (3) `POST TravelSchedule/AddV2` (urlencoded, header
`language: zh-tw`) with the payload in protocol.md: `TotalDay` is the
inclusive day count, `TrafficType` from `--traffic` (default `Custom`;
accepted: `Custom`, `Transit`, `Driving`, `Walk`, `PublicTransport`),
`LocationKey[]` from each `--location` (e.g. `7,7,0`; at least one is
required, chicTrip rejects a trip without a destination), `CoverMediaId`
the first entry of `GET TravelSchedule/GetSystemCoverList` (chicTrip
rejects an empty one). Validation before any call: dates parse, end >=
start, span <= 60 days, name non-empty, at least one `--location`.

```
trip:
  id: 3c1d...
  name: Tokyo temples
  start: 2026/10/01
  end: 2026/10/03
  days: 3
  update_time: 1789877000
help[2]:
  Run `chictrip-axi trip add 3c1d... --day 1 --poi <poi-id> --poi <poi-id>` to fill day 1
  Run `chictrip-axi poi search "<keyword>"` to find POI ids
```

### trip view

`VerifyUpdateTime` then `GET TravelScheduleDetail/Get` (protocol.md,
"Reading my trips"). Same table as `tour view` plus `tsd_id`, which
`trip remove` needs. `--day N` keeps one day.

```
trip:
  id: 3c1d...
  name: Tokyo temples
  start: 2026/10/01
  end: 2026/10/03
  days: 3
  permission: Owner
  update_time: 1789877000
stops[5]{day,seq,arrive,stay_min,name,type,tsd_id,poi_id}:
  1,1,09:00,60,Senso-ji Kaminarimon,poi,b2d1...,8a48...
  ...
help[2]:
  Run `chictrip-axi trip add 3c1d... --day <n> --poi <poi-id>` to add stops
  Run `chictrip-axi trip remove 3c1d... --stop <tsd_id>` to remove one
```

An empty trip prints `stops[0]:` and the `trip add` hint.

### trip add

Per protocol.md "Add a POI to a day", batched: (1) `trip view` data
once (current `update_time` and the day's existing `poi_id`s);
validate `--day` is within `days`; (2) for each `--poi` in order: skip
when the POI is already in that day unless `--allow-duplicate`;
`GET Poi/GetPoiById?id=` for `TsdName` and `TsdCoverMediaId`
(`not_found` if the POI does not exist, before anything is written);
`GET TravelScheduleDetail/GetAddWhere`; pick the slot: `--position
last` (default) is the last slot of the day, `best` is the API's
`isBestOfDay` (else `isBestOfAll`, else the first); `POST
TravelScheduleDetail/Add` with the current update time, then chain the
returned time. On `004` re-verify and retry that POI once. Stop at the
first hard failure and report what was added so far.

```
trip_id: 3c1d...
day: 1
added[2]{poi_id,name,tsd_id,seq}:
  8a48...,Senso-ji Kaminarimon,b2d1...,1
  0dbf...,Kamakura Great Buddha,c9e2...,2
skipped[1]{poi_id,reason}:
  edd5...,already in day 1
update_time: 1789877100
help[2]:
  Run `chictrip-axi trip view 3c1d... --day 1` to see the day
  Run `chictrip-axi trip remove 3c1d... --stop <tsd_id>` to undo
```

If every POI was skipped, `added[0]:` and exit 0 (idempotent). `seq`
is the position in the day after the add when the response carries
it; otherwise the row order.

### trip remove

`trip view` data once to map each `--stop` to its day, then
`DELETE TravelScheduleDetail/Delete` per stop with the chained update
time. A `--stop` that is not in the trip is reported under
`skipped[]{tsd_id,reason}` with `not in trip (no-op)`; exit 0 when
nothing failed.

```
trip_id: 3c1d...
removed[1]{tsd_id,name,day}:
  c9e2...,Kamakura Great Buddha,1
update_time: 1789877200
help[1]:
  Run `chictrip-axi trip view 3c1d...` to confirm
```

### trip delete

`DELETE TravelSchedule/Delete` with `id`. Prints `deleted: <id>` and
`help` pointing at `trip list`. A trip that is already gone (011 or
the API's not-found message) is `deleted: <id>` with `note: already
absent (no-op)`, exit 0. No confirmation prompt (AXI: nothing prompts);
the destructive intent is the explicit verb.

### tour list

`ExpertTour/PopularRanking?page=&pageSize=` with `pageSize = --limit`
(default 20); `--curated` switches to `ExpertTour/Exclusive` (8 editor
picks, no paging).

```
count: 20
page: 1
next_page: 2
tours[20]{id,name,destination,expert,likes,used}:
  daebf5f2-...,Tokyo 7 days 6 nights notes,Japan,Mamo,4014,18052
help[3]:
  Run `chictrip-axi tour view <id>` to read an itinerary day by day
  Run `chictrip-axi tour copy <id>` to copy one into my trips
  Run `chictrip-axi tour list --page 2` for the next page
```

`--fields` extras: `tags` (joined with `|`) and, for `--curated`,
`introduction`.

### tour view

`ExpertTour/TourV2?travelScheduleId=`. Default prints every day;
`--day N` keeps one.

```
tour:
  id: 8c9b156f-...
  name: Hakone and Kamakura 3 days
  destination: Japan
  days: 3
  start: 2024/01/17
  end: 2024/01/19
  expert: Suiryoshuku
  likes: 1310
  used: 4711
  youtube: https://youtu.be/...
  introduction: ... (truncated, 1204 chars total)
stops[31]{day,seq,arrive,stay_min,name,type,city,poi_id}:
  1,1,13:55,60,TPE Taoyuan International Airport,flight,Taoyuan,2959f714-...
  ...
help[3]:
  Run `chictrip-axi poi view <poi_id>` for a stop
  Run `chictrip-axi tour copy 8c9b156f-...` to copy this itinerary into my trips
  Run `chictrip-axi tour view 8c9b156f-... --full` for notes and traffic per stop
```

`--full` adds `note`, `traffic`, `traffic_min`, `flight` columns, the
whole `introduction`, and `highlights[N]{poi_id,name,rating,visits}`
from `overview.highlightPoiList`.

### tour copy

Member only. `POST ExpertTour/TravelScheduleCopy` with
`travelScheduleId`, then `trip list` to find the newest trip (the copy)
and print it as `trip:` (id, name, start, end, days, update_time) with
`source: <tour-id>`. `help`: `trip view <id>`, `trip add`. Not
idempotent by nature (the app allows several copies); say so in
`--help`.

### poi search

`PoiSearch/SearchByKeyword?keyword=&centerLongitude=&centerLatitude=`
(0,0 without `--near`). Default `--limit 20`.

```
count: 12 of 12
pois[12]{id,name,category,rating,city,area}:
  edd5509c-...,Azumabashi pier,enterTainment,4,Tokyo,Sumida
help[2]:
  Run `chictrip-axi poi view <id>` for hours, address, and description
  Run `chictrip-axi trip add <trip-id> --day <n> --poi <id>` to put one in a trip
```

`--fields` extras: `reviews`, `favorites`, `visits`, `lat`, `lng`,
`address`, `place_id`, `country`. `hasNextPage` maps to `more: true`.
Empty: `count: 0`, `pois[0]:`, a hint to shorten the keyword or add
`--near`.

### location search

`ExpertTour/SearchLocation?keyword=`, guest token is enough. Default
`--limit 20`. `key` is the `country,city,area` string `trip create`
takes verbatim.

```
count: 2
locations[2]{name,full_name,key}:
  Tokyo,Japan/Tokyo,"7,7,0"
  Shinjuku,Japan/Tokyo/Shinjuku,"7,7,6"
help[1]:
  Run `chictrip-axi trip create --name "<name>" --start <date> --end <date> --location <key>` to file a trip there
```

Empty: `count: 0`, `locations[0]:`, a hint to try a shorter name.

### poi view

`Poi/GetPoiById?id=`. No `help` unless truncated.

```
poi:
  id: 8a48a94c-...
  name: Senso-ji Kaminarimon
  category: enterTainment
  rating: 4.5
  reviews: 36177
  favorites: 22142
  visits: 220669
  address: 2 Chome-3-1 Asakusa, Taito City, Tokyo 111-0032
  country: Japan
  city: Tokyo
  area: Taito
  lat: 35.7111163
  lng: 139.7963656
  phone: 03-3842-0181
  url: http://www.senso-ji.jp/guide/guide01.html
  hours[7]: monday 00:00-24:00,tuesday 00:00-24:00,...
  tags[3]: Tokyo top 1 sight,historic site,landmark
  description: ... (truncated, 812 chars total)
  media: 54
  tickets: 3
help[1]:
  Run `chictrip-axi poi view 8a48a94c-... --full` for the full description, media urls, and tickets
```

`--full` expands `media[N]{type,source,url}` and
`tickets[N]{title,price,url}` and prints `description` whole.

## Module layout (src/)

```
main.rs          parse argv (clap errors -> usage error on stdout, exit 2), run, exit code
cli.rs           clap tree (Cli, Command enums per noun), global flags, home view assembly
auth.rs          AuthFile {access_token, refresh_token, member_id} load/save (0600);
                 TokenSource resolution (flag > env > file > guest); JWT exp decoding;
                 GUEST_TOKEN constant with provenance comment
api/mod.rs       Client { base_url, token source, timeout }: get(path, params),
                 post_form / put_form / delete_form (urlencoded), the envelope parser
                 (apiStatus or ApiStatus), status -> AxiError mapping, 003 -> refresh
                 and retry once, 004 -> surfaced as Conflict for callers that retry
api/types.rs     serde structs for the endpoints used (unknown fields ignored)
api/trips.rs     the multi-call recipes: current_update_time, default_label_id,
                 add_stop (GetPoiById + GetAddWhere + Add), remove_stop, copy_tour
output.rs        Document builder (count/page lines, tables, truncate(text, 500),
                 help lines, --fields filter) and render_toon / render_json
error.rs         AxiError { code, message, help, request_id } + exit_code()
commands/        auth.rs, trip.rs, tour.rs, poi.rs: fetch -> Document; no formatting
                 or error text of their own
```

Tests: unit tests per module on trimmed fixtures under
`tests/fixtures/`; one integration test that starts a
`std::net::TcpListener` fixture server (records requests, so form
bodies and chained update times can be asserted) and runs the binary
with `CHICTRIP_AXI_BASE_URL` and `CHICTRIP_AXI_AUTH_FILE` pointing at
temp paths: exit codes, TOON snapshots, error shapes, unknown flag ->
exit 2 with `flags[]`, guest -> `auth_required` without any request,
003 -> refresh -> retry, 004 -> retry. No test touches the network or
the user's HOME.

## Decisions (confirmed by the user, 2026-09-18)

- D1 crates: `ureq = { version = "3", features = ["json"] }` (sync,
  rustls + ring, gzip; rust-version 1.85; the release workflow builds
  musl with `cross`, so ring cross-compiles), `serde` (derive),
  `serde_json`, `toon-format = { version = "0.5", default-features =
  false }` (official Rust implementation, spec v3.0; our flat shapes
  encode identically under v4.1; the default `cli` feature would pull
  ratatui). Rejected: reqwest (tokio), etoon (young, sonic-rs),
  hand-rolled encoder.
- D2 auth: embedded guest token for reads; member token in the auth
  file with automatic refresh (section "Authentication").
- D3 scope: the write vertical slice above (14 commands + home view;
  `location search` was added during implementation, see below;
  milestone 3 took the tree to 18 and milestone 4 to 24).
- D4 live verification: the user places their tokens in
  `~/.config/chictrip-axi/auth.json`; the implementing agent may create
  a clearly named test trip in that account, exercise add/remove/view,
  and delete it. The token file never enters the repo.

## Implementation notes for the executing agent

- Read `.agents/skills/axi/SKILL.md` and the TOON spec (SPEC.md v4.1,
  https://github.com/toon-format/spec/blob/main/SPEC.md) before the
  output layer.
- Send `osType: web` and `language: zhtw` on every request (`language:
  zh-tw` on AddV2/UpdateV3/Copy as the app does); accept `apiStatus`
  and `ApiStatus`.
- Send every required query parameter (protocol.md, "The non-empty
  request body trap").
- Bodies: send `application/x-www-form-urlencoded` everywhere first
  (ASP.NET form binding accepts it where the app sends multipart);
  encode arrays as `Name[]=v` like axios; ureq 3 needs
  `force_send_body()` to put a body on DELETE. If the live test rejects
  a urlencoded body on an endpoint the app sends as multipart, build a
  multipart body for that endpoint (no new crate needed).
- Validate everything local (`--near`, dates, `--limit` 1..=200,
  `--page` >= 1, member-token presence) before constructing the client.
- Version fast path: clap handles `-V/--version` before any client or
  file access; `main` does no eager network or file I/O.
- `TravelScheduleDetail/Get` for own trips has the same
  `{travelScheduleInfo, dayList}` shape as the guest-readable
  `TravelScheduleDetail/Preview?TravelScheduleId=` (protocol.md, "Trip
  previews"), so `trip view` can be developed and tested against that
  fixture before a member token is available. Milestone 3 turned
  that endpoint into `trip preview` and reuses the same fixture for both
  commands' tests. The `Add` and
  `TravelScheduleCopy` responses were not observed; confirm them in the
  live test and record them in protocol.md.
- Repo rules: ASCII-only sources and comments, `just gate` green before
  each commit, conventional commits with scopes `build` (Cargo.toml),
  `feat(api)`, `feat(auth)`, `feat(output)`, `feat(trip)`, `feat(tour)`,
  `feat(poi)`, `feat(cli)`, `test`, `docs`; never push.

## Live-run corrections (2026-09-18)

What the implementation and the live run against api.chictrip.com.tw
changed relative to the text above. protocol.md carries the API-side
evidence.

- `trip create` needs three real values or chicTrip answers the generic
  `002 A non-empty request body is required`: a `CoverMediaId` from
  `TravelSchedule/GetSystemCoverList` (the app preselects the first),
  the label id, and at least one `LocationKey[]`. Hence `--location` is
  required and `location search` (ExpertTour/SearchLocation) joined the
  milestone so an agent can obtain a key. `destinationList` is not sent.
- `tsdType` values are `basic` and `flight`, not `poi`; `type` shows
  them verbatim.
- `count` prints `N of M` only when `--limit` trimmed rows; a bare
  number otherwise (the general rule wins over the `12 of 12` example).
- `help` is a TOON inline primitive array (`help[2]: a,b`), not one
  line per entry as the examples above draw it; help sentences avoid
  commas so they need no quotes. The encoder quotes any value holding
  `:` `-` `[` `]` `{` `}`, so UUIDs, `HH:MM` times, and most `help`
  lines print quoted; the output is valid TOON and decodes back.
- `--fields` targets the table or object each command names as primary
  (`poi:` for `poi view`, `stops` for `trip view` and `tour view`, the
  list table otherwise), not "the first table".
- `trip add` re-reads the trip once after the last write to report the
  new stops' `tsd_id` and `seq`: the `Add` response carries
  `data.tsdInfo.id` but no position.
- `trip delete` on a trip that is already gone prints no `note`:
  chicTrip answers `001 true` for an unknown id, so the CLI cannot tell.
  Exit 0 either way.
- The home view prints `error` and `message` lines when its live call
  fails, above the command index, so the agent sees why.
- `src/datetime.rs` (UTC calendar maths) exists instead of a date
  crate; the shared table builders live in `commands/mod.rs` so the
  noun modules do not depend on each other.

## Milestone 3: trip previews and session integration

Approved by the user on 2026-09-18 and implemented the same day. Three
parts: `trip preview`, the two `setup` commands, and an evaluation of
the encoder's quoting that deliberately changed nothing.

### trip preview

`chictrip-axi trip preview <trip-id> [--day N]` reads any trip by id.
The id is usually the `preViewTravelId` of
`https://www.chictrip.com.tw/?action=preView&preViewTravelId=<id>`.

- chicTrip gates Preview on nothing but the id. Live on 2026-09-18 a
  private trip created seconds earlier in the test account was readable
  with the guest token, and answered `011 TravelSchedule has been
  deleted` once deleted. "Shared by link" is a UI notion, not an API
  one, so every surface says "any trip by id" and never promises that
  an unshared trip stays private.

- Endpoint `TravelScheduleDetail/Preview`, parameter `TravelScheduleId`
  with a capital T (`TravelScheduleDetail/Get` spells the same thing
  `travelScheduleId`). The embedded guest token is enough, so this is
  the one `trip` command that does not call `member_client`.
- One request. `trip view` first asks `VerifyUpdateTime` because a
  mutation needs the trip's `updateTime`; a preview writes nothing, so
  that call would buy nothing.
- The header is the first five lines of `trip view`'s: `id`, `name`,
  `start`, `end`, `days`. `permission` and `update_time` are dropped.
  `permission` is the owner's value (a guest reading somebody's trip sees
  `Owner`), so printing it would tell the agent it may write; the
  `updateTime` chain only feeds mutations a viewer cannot make. The two
  commands share `trip_header`, and `trip view`'s output is unchanged
  byte for byte.
- Stops come from `stops_table(&days, false, false)`, so the `tsd_id`
  column is out as well: `trip remove` is the only command that takes
  one and it needs a trip of my own.
- `--day N` goes through the shared `select_days`, so a day outside the
  itinerary is `not_found` naming the day count, decided after the one
  request rather than with a second.
- Errors: `011 TravelSchedule has been deleted` is what an unknown or
  deleted id answers and `api/mod.rs` already maps 011 to `not_found`. A
  missing or malformed `TravelScheduleId` answers the generic `002 A
  non-empty request body is required` and stays `api_error`; validating
  the id shape locally is out of scope (see Scope).
- `help` is two lines: `poi view <poi_id>` for a stop, and
  `trip add <my-trip-id> --day <n> --poi <poi_id>` to copy a stop into a
  trip of my own.

Against the `trip_detail.json` fixture (three days, one stop):

```
trip:
  id: "3c1d0a2e-1111-4111-8111-111111111111"
  name: Tokyo temples
  start: 2026/10/01
  end: 2026/10/03
  days: 3
stops[1]{day,seq,arrive,stay_min,name,type,city,poi_id}:
  1,1,"09:00",60,Azumabashi pier,basic,Tokyo,"edd5509c-d852-41d1-9d27-0139d6d9f8f5"
help[2]: "Run `chictrip-axi poi view <poi_id>` for a stop","Run `chictrip-axi trip add <my-trip-id> --day <n> --poi <poi_id>` to copy a stop into my own trip"
```

### setup skill and setup hooks

AXI section 7 asks for both a session hook (ambient context, live state,
per-session token cost) and an installable skill (on-demand, static, no
per-session cost), presented as two ways to the same end of which a user
needs one. `setup` is a noun of its own so that neither is ever
installed as a side effect of an ordinary command, and `commands::setup`
is dispatched without a `Context`, which makes "touches no network" a
property the compiler enforces.

`setup skill [--check] [--out PATH]` writes
`skills/chictrip-axi/SKILL.md` (the flat layout `npx skills add` reads).
The compromise on content: the prose lives in
`src/commands/skill_template.md`, compiled in with `include_str!`, and
the generator substitutes two placeholders, `{{description}}` with
`cli::DESCRIPTION` and `{{commands}}` with one bullet per `COMMAND_INDEX`
row. Fully generating the file would mean encoding every sentence in
Rust; fully hand-writing it would let the command list drift. No version
number is embedded, so cutting a release never rewrites the file.

| Invocation | Result |
|---|---|
| `--check`, file missing | `not_found`, help names `setup skill`, exit 1 |
| `--check`, identical | `status: current`, exit 0 |
| `--check`, different | `conflict`, help names `setup skill`, exit 1 |
| write, identical | no write, `status: unchanged` |
| write | `status: written` |

`cargo run -- setup skill --check` is the last step of `just gate` and a
step of the CI `test` job, so a hand edit or a stale copy fails the
build. Agents install the published skill with `npx skills add
qq88976321/chictrip-axi --skill chictrip-axi`; `--skill` is required
because the repository also vendors the `axi` design skill.

`setup hooks [--user] [--remove]` targets Claude Code only. It edits
`.claude/settings.json` under the current directory, or
`$CLAUDE_CONFIG_DIR/settings.json` (default `~/.claude`) with `--user`,
and appends one group to `hooks.SessionStart`:

```json
{"hooks": [{"type": "command", "command": "chictrip-axi --timeout 5", "timeout": 10}]}
```

- No `matcher` key. Omitting it is how Claude Code spells "every start
  reason": startup, resume, clear, compact, fork.
- The command is the bare binary name when the first `chictrip-axi` on
  `PATH` canonicalizes to this executable, and the canonical absolute
  path otherwise (double quoted when it contains a space, since the
  command runs through `sh -c`). A bare name keeps a global install
  portable between machines; a pinned path stops the hook running some
  other build.
- Idempotent: a second run with the same command is `unchanged` and
  writes nothing. Repair: a hook of ours whose command has gone stale
  has only its `command` replaced, so a `matcher` or `timeout` the user
  added survives. Remove: only hooks whose first shell word is named
  `chictrip-axi` are taken out, an emptied group is dropped, and an
  emptied `SessionStart` and `hooks` go with it; nothing to remove is
  `absent`, exit 0.
- A settings file that is not JSON, or whose `hooks` or
  `hooks.SessionStart` is not the documented shape, is a `conflict` that
  names the file. The command never rewrites a file it cannot read.
- Known behaviour, accepted: the home view exits 1 when its one live
  call fails, and Claude Code does not inject the stdout of a hook that
  exited non-zero. A session started while chicTrip is unreachable gets
  no context at all, not even the command index the CLI still prints.
- `serde_json` is declared with `features = ["preserve_order"]` so a
  settings file keeps its key order across an edit. `toon-format`
  already enabled it and `indexmap` was already in `Cargo.lock`; naming
  it stops that from being an accident of the dependency graph.

### TOON quoting: evaluated, kept toon-format 0.5 unchanged

Almost every id in this CLI's output is printed quoted, which costs two
tokens a row. The evaluation:

- TOON spec v4.1 (2026-07-26) section 7.2 requires quoting only for a
  value that *starts* with `-`. `toon-format` 0.5.0 puts `-` in its
  `STRUCTURAL_CHARS`, so every string containing a hyphen is quoted:
  every UUID. That is the crate being stricter than the spec. The
  quoting of `09:00` is the spec's own rule, not the crate's.
- The one crate tracking v4.1, `etoon` 0.8.0, is encoder-only, takes
  JSON bytes rather than a serde value, depends on `sonic-rs` (a risk
  for the musl and aarch64 release builds), and has a single star. Not
  a swap worth making for a personal tool.
- A short-id lookup table (print `t1`, accept `t1` back) was rejected
  outright. `trip add --poi`, `trip remove --stop`, `trip view <id>`,
  and every `help` line hand ids back to the agent verbatim; a mapping
  would have to survive between processes, which means per-machine
  state, which means an id that means different things on two machines.
- The fallback, if the cost ever matters: about 150 lines in this repo
  encoding the four node kinds the output layer produces (scalar, list,
  table, object) to v4.1 quoting rules, replacing `toon_format::encode_default`
  at the one call site in `output.rs`.

Decision: change nothing. The CLAUDE.md rule stands - the encoder's
quoting is the encoder's rule, and no command hand-rolls around it.


## Milestone 4: edit what is already in a trip

Approved by the user on 2026-09-19 (decisions D5 to D8 below) and
implemented the same day. Six parts, all over endpoints exercised live
on the probe trip `axi-probe-m4` in the test account: `--full` on the
two read commands, exact insertion for `trip add`, and the verbs `trip
edit`, `trip note`, `trip leg` with `trip traffic`, `trip move`, plus
`poi create`. The tree grows to twenty-four commands.

### trip view --full and trip preview --full

- No new request. `stops_table(days, full, with_tsd_id)` already knew
  the detail columns; `--full` promotes `note`, `traffic`,
  `traffic_min`, `depart`, `category` (the row's `categoryIcon`:
  enterTainment, food, shop, moon, rentCar, train, plane,
  chargingPoint, other, pin, or takeOff/transfer/landing on a flight
  row) and `flight` from `--fields` extras to printed columns. `type`
  keeps meaning `basic` or `flight`. `arrive` prefers the pinned
  `customArrivalTime` when the row says that is the one the app shows,
  and `depart` is printed only when a departure is pinned, because the
  computed one is arrival plus stay.
- `flight` prints `flightNumber`, which the web app never writes; it
  is populated on expert tours only and stays for `tour view` parity.
- `trip preview --full` shows the same columns without `tsd_id`.
  Preview lags a write by seconds and answers `note: ""` for the trip
  header, so it is never the read-back oracle; `trip view` is.
- The stops schema without `--full` is unchanged byte for byte. The
  `trip view` header gains a `note` field when the trip has one.

```
trip:
  id: "3c1d0a2e-1111-4111-8111-111111111111"
  name: Tokyo temples
  start: 2026/10/01
  end: 2026/10/03
  days: 3
  permission: Owner
  update_time: 1789710463
  note: Buy the 72h subway pass at Narita
stops[3]{day,seq,arrive,stay_min,name,type,tsd_id,city,poi_id,note,traffic,traffic_min,depart,category,flight}:
  1,1,"09:00",60,Azumabashi pier,basic,"b2d11753-aaaa-4aaa-8aaa-aaaaaaaaaaaa",Tokyo,"edd5509c-d852-41d1-9d27-0139d6d9f8f5",null,Custom,0,null,pin,null
  1,2,"10:30",90,"Senso-ji",basic,"c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb",Tokyo,"8a48a94c-495f-44da-be0d-e1d7564f2b07","Reservation 19:00",Transit,12,"12:00",enterTainment,null
  1,3,"12:25",45,Tokyo Skytree,basic,"d0000000-cccc-4ccc-8ccc-cccccccccccc",Tokyo,"f1111111-d852-41d1-9d27-0139d6d9f8f5",null,Custom,25,null,enterTainment,null
```

### trip add: --position first and --after

- `GetAddWhere` answers one slot per day: `start`, one named after
  each stop EXCEPT the first, and `end` (an empty day answers a single
  slot named `first`). A slot named after a stop inserts the new one
  IN FRONT of it, which the live run corrected (see below), so
  `--position first` is the `start` slot and `--after X` is the slot
  of the stop that FOLLOWS X, or `end` when X is last.
- An anchor that is not in the chosen day is a `not_found` decided
  from the one `trip view` read, before any write. `--after` and
  `--position` together are a usage error (clap).
- A batch holds one slot for every POI, except that `--position
  first` switches to the day's old first stop after its first insert.
  Either way `--poi A --poi B` keeps A before B.
- Summary changes from "Append POIs" to "Add POIs": the verb no longer
  implies the end of the day.

### trip edit

- Two requests per stop after `VerifyUpdateTime`: `GET
  TravelScheduleDetail/GetEditInfo` (which needs the CURRENT update
  time; `0` is a 004 here) for the sheet, then `PUT
  TravelScheduleDetail/Update` with every field of the sheet and the
  edited ones replaced, then a second `GetEditInfo` to print what
  chicTrip now holds. Chained update time; a 004 is re-verified and
  retried once.
- Flags: `--stay MIN` (0 to 1440), `--arrive HH:MM|auto`, `--depart
  HH:MM|auto` (`auto` sends `IsUseCustom*Time=0` and an empty time),
  `--category TYPE` (an icon token from the sheet's own
  `categoryList`; takeOff, transfer or landing turns the row into
  `type: flight`), `--name TEXT`. At least one is required. Time and
  minute syntax are validated before any network call.
- No-op: when every requested value already equals the sheet, nothing
  is written, `changed[0]:` and `note: already as requested (no-op)`,
  exit 0.
- Errors: an unknown `--stop` is a `not_found` (chicTrip answers `002
  TSD Id not found`); an unknown `--category` is a `usage` error that
  lists the twelve live tokens, after the one GET that learned them.

```
trip_id: "3c1d0a2e-1111-4111-8111-111111111111"
stop:
  tsd_id: "c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
  name: "Senso-ji"
  category: food
  type: basic
  arrive: "10:30"
  arrive_custom: true
  depart: "12:00"
  depart_custom: true
  stay_min: 120
changed[2]: category,stay_min
update_time: 1789710700
help[2]: "Run `chictrip-axi trip view 3c1d0a2e-1111-4111-8111-111111111111 --full` to see every stop with its times","Run `chictrip-axi trip note 3c1d0a2e-1111-4111-8111-111111111111 --stop c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb --set \"<text>\"` to attach a note"
```

### trip note

- Without `--set` or `--clear` it reads, from the one
  `TravelScheduleDetail/Get` the command already does: the row's
  `note` for a stop, the header's `note` for the trip. Printed in
  full, because a note is the detail view of itself. With `--set TEXT`
  it is `PUT TravelSchedule/UpdateNote` or `PUT
  TravelScheduleDetail/UpdateNote`; `--clear` is `--set ""`. The two
  together are a usage error.
- Per-day notes do not exist on the platform, so there is no `--day`.
- No-op: the stored text already equals the request, so nothing is
  written, `status: unchanged (no-op)`, exit 0.
- A write is not read back: the `001` is the oracle and Preview lags.

```
trip_id: "3c1d0a2e-1111-4111-8111-111111111111"
stop_id: "c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
name: "Senso-ji"
note: "Reservation 19:00"
help[1]: "Run `chictrip-axi trip note 3c1d0a2e-1111-4111-8111-111111111111 --stop c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb --set \"<text>\"` to change it"
```

### trip leg and trip traffic

- A leg belongs to the stop it ARRIVES at; a day's first stop has none
  (`tsdRouteDetailId` is null). `--stop` on a first stop is a
  `not_found` that says so and names the next stop, before any write.
- Listing (no setter flag): `GET
  TravelScheduleDetailRoute/GetRouteList` for `--mode`, else the
  stop's own mode when that one is routable, else driving. `--mode`
  never writes. Transit rows carry a fare and no summary; driving rows
  carry a summary and no fare.
- Setting: `--route ROUTE-ID` is `SetRoute`, `--custom MIN [--note]`
  is `SetCustomRoute`, `--flight MIN [--note]` is `SetFlightRoute`.
  The three are mutually exclusive and `--note` requires one of the
  last two. Mode tokens: driving, transit, walking, scooter
  (chicTrip's `TwoWheeler`).
- No-op: the leg already is what was asked (the route is `isSelected`,
  or the stored free-form leg has the same type, duration and note):
  nothing written, exit 0.
- `trip traffic <trip> --day N --mode M [--recompute]` is `PUT
  TravelScheduleDetail/SetDefaultRouteAndTsdAllDay` with
  `isForceUpdateTsdRoute` = `--recompute`. Without it only the day's
  default changes and new legs pick it up; with it every leg of the
  day is recomputed, which overwrites hand-set ones, and the `--help`
  says so. The answer carries the whole day, so there is no re-read.
- Flight details: chicTrip has flight number and terminal fields on
  the row but the web app never writes them, so the convention is
  `--category takeOff|landing` on the stop plus the flight code in the
  leg note (`--flight 180 --note "BR198 TPE-NRT"`).

```
trip_id: "3c1d0a2e-1111-4111-8111-111111111111"
stop:
  tsd_id: "c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
  name: "Senso-ji"
  day: 1
  from: Azumabashi pier
  traffic: Transit
  traffic_min: 12
mode: Transit
count: 2
routes[2]{route_id,minutes,km,summary,fare,selected}:
  "t-1",14,3.2,null,JPY 180,true
  "t-2",22,3.9,null,JPY 210,false
```

```
trip_id: "3c1d0a2e-1111-4111-8111-111111111111"
day: 1
traffic: Driving
stops[3]{seq,arrive,stay_min,name,tsd_id,traffic,traffic_min}:
  1,"09:00",60,Azumabashi pier,"b2d11753-aaaa-4aaa-8aaa-aaaaaaaaaaaa",Custom,0
  2,"10:08",90,"Senso-ji","c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb",Driving,8
  3,"11:50",45,Tokyo Skytree,"d0000000-cccc-4ccc-8ccc-cccccccccccc",Driving,12
update_time: 1789710800
```

### trip move

- `PUT TravelScheduleDetail/Sort` with the whole TARGET day's ids in
  their new order, computed locally from the read the command already
  does. `--after TSD-ID` or `--position first|last`; one of them is
  required (clap group), and `--after` naming the stop itself is a
  usage error.
- `--day N` moves the stop into another day with the same one call:
  the live run answered the open question, and the stop keeps its id,
  its note, its stay and its pinned times, so the app's Copy plus
  Delete recipe is not needed and `copy_stop` was never built.
- No-op: already in that place in that day, so nothing is written.

```
trip_id: "3c1d0a2e-1111-4111-8111-111111111111"
moved:
  tsd_id: "d0000000-cccc-4ccc-8ccc-cccccccccccc"
  name: Tokyo Skytree
  from_day: 1
  day: 1
  seq: 2
stops[3]{seq,arrive,stay_min,name,tsd_id}:
  1,"09:00",60,Azumabashi pier,"b2d11753-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
  2,"12:25",45,Tokyo Skytree,"d0000000-cccc-4ccc-8ccc-cccccccccccc"
  3,"10:30",90,"Senso-ji","c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
update_time: 1789711100
```

### poi create

- `POST Poi/AddCustomPoiForWeb` with `name, categoryId, longitude,
  latitude, address, description`, urlencoded like every other write:
  the web app sends multipart with a media part, but the live run
  showed neither is required, so no multipart transport was built.
  `categoryId` is resolved from `--category` through `GET
  PoiClassification/GetCustomPoiCategory`, the seven icons the app's
  own form offers, and the default is `enterTainment` because that
  endpoint has no `other`.
- Validation before the network: `--at` parses like `--near` (lat -90
  to 90, lng -180 to 180) and `--name` must be non-empty.
- NOT idempotent, and the `--help`, a `note:` line, the skill and the
  README all say so: a private place is not searchable, so the CLI
  cannot find an existing one, and chicTrip has no delete endpoint.
  Every run files a permanent row.

```
poi:
  id: "7d2e5a10-6666-4666-8666-666666666666"
  name: Aunt Mei's flat
  category: food
  lat: 35.7111
  lng: 139.7963
  address: "2-3-1 Asakusa"
note: private to this account; not searchable; chicTrip cannot delete it
help[2]: "Run `chictrip-axi trip add <trip-id> --day <n> --poi 7d2e5a10-6666-4666-8666-666666666666` to put it in a trip","Run `chictrip-axi poi view 7d2e5a10-6666-4666-8666-666666666666` to read it back"
```

### Decisions (confirmed by the user, 2026-09-19)

- D5 scope: all six groups in one milestone (view `--full`, add
  positions, edit, note, leg with traffic, move, poi create). Best
  sort, day operations and trip-level edits stay out.
- D6 verbs: flat verbs under `trip` addressed by `--stop TSD-ID`
  (`trip edit/note/leg/move ... --stop`), the way `trip remove --stop`
  already worked; no `trip stop <verb>` sub-noun. `trip traffic` takes
  `--day` because its unit is the day.
- D7 live verification: the implementing agent ran the per-command
  script against the test account (a scratchpad copy of
  `auth.test.json` through `CHICTRIP_AXI_AUTH_FILE`, so a token
  refresh never rewrites the real file) on the probe trip
  `axi-probe-m4`, using day 4 as the scratch day and restoring days 1
  to 3. `trip view` is the oracle, never `trip preview`.
- D8 platform facts that shaped the commands: a leg belongs to the
  arriving stop and a day's first stop has none; Preview lags a write
  and hides the trip note; per-day notes do not exist; a flight is a
  TsdCategory on the stop plus a Flight leg with the code in its note,
  because the row's flight fields are never written by the web app;
  every write is form-encoded and a JSON body gets the same generic
  002 as a missing field; the stop table's `type` column is `tsdType`,
  so the icon vocabulary is called `category` everywhere (the flag,
  the column, and `poi search`).

### Live-run corrections (2026-09-19)

- `addWhereId` names the stop the new one is inserted IN FRONT OF, not
  the stop it follows, and an empty day answers a single slot named
  `first` rather than `start` and `end`. So `--position first` sends
  `start` and `--after X` sends the id of the stop after X. The
  `Position` enum became `Last | Best | Slot(addWhereId)` and the
  command translates, instead of the designed `First | After(id)`.
- `TravelScheduleDetail/Add` answers `{tsdInfo, travelScheduleUpdateTime}`
  and `tsdInfo.id` is the new stop, so the batch chains on the id
  rather than guessing from the POI id. An empty `TsdCoverMediaId`
  makes the same call answer the generic 002.
- `GetEditInfo` with an unknown `tsdId` answers `002 TSD Id not found`
  with a null payload, not a 001 with no data, so the not_found is
  mapped from that message inside the recipe and the shared apiStatus
  table is untouched.
- `TravelScheduleDetail/Sort` accepts `MoveOutDay != MoveInDay` and
  moves the stop keeping its id, note and stay, so `trip move --day`
  is one call and `copy_stop` was never built.
- `Poi/AddCustomPoiForWeb` accepts `application/x-www-form-urlencoded`
  with no media part, so `Method::PostMultipart` and the hand-built
  multipart body were never built either.
- `PoiClassification/GetAll?page=1` answers an object
  `{page, list[], hasNextPage}` mixing Category and Tag rows, so
  `poi create` resolves its `--category` through
  `GetCustomPoiCategory` and defaults to `enterTainment`.
- `GetRouteList` answers `null`, not `[]`, for an empty route list,
  which `#[serde(default)]` does not cover; the two list fields go
  through a `null_as_empty` helper in `types.rs`.
- `trip edit` prints `changed` in field order (name, category,
  stay_min, arrive, depart), so the design's `changed[2]:
  stay_min,category` reads `category,stay_min`.
- The `trip view` header gained `note`; only the stops schema is
  unchanged byte for byte.
