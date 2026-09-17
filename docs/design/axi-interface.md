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
HTTP/output/error/auth layers. Out (later milestones): rankings,
nearby, comments, categories (designed in git history at `2834aac`), stop reordering and notes, `trip preview <id>` for shared trips,
`skills/chictrip-axi/SKILL.md` generation with its `--check` gate,
`setup hooks`.

## Command tree

```
chictrip-axi                                   home view
chictrip-axi auth set (--access-token T --refresh-token R --member-id M | --from-json FILE|-)
chictrip-axi auth status
chictrip-axi auth clear
chictrip-axi trip list [--limit N]
chictrip-axi trip create --name NAME --start DATE --end DATE --location KEY... [--traffic MODE] [--duplicate]
chictrip-axi trip view <trip-id> [--day N]
chictrip-axi trip add <trip-id> --day N --poi POI-ID... [--position last|best] [--allow-duplicate]
chictrip-axi trip remove <trip-id> --stop TSD-ID...
chictrip-axi trip delete <trip-id>
chictrip-axi tour list [--curated] [--page N] [--limit N]
chictrip-axi tour view <tour-id> [--day N] [--full]
chictrip-axi tour copy <tour-id>
chictrip-axi poi search <keyword> [--near LAT,LNG] [--limit N]
chictrip-axi poi view <poi-id> [--full]
chictrip-axi location search <keyword> [--limit N]
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

- Commands under `trip`, plus `tour copy`, need a member token. When
  the effective token is the embedded guest, they fail locally with
  `auth_required` before any network call (the guest "member" would
  otherwise answer with demo data).
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
description: Agent-first CLI for chicTrip: search places, read expert itineraries, build trips in your account
auth: member
trips[3]{id,name,start,end,days}:
  fd4db85c-...,Tokyo temples,2026/10/01,2026/10/03,3
  ...
commands[15]{command,summary}:
  auth set,Store a member token copied from the browser
  auth status,Show which token is in use and whether it works
  auth clear,Forget the stored member token
  trip list,My trips (newest first)
  trip create --name --start --end,Create an empty trip
  trip view <trip-id>,Stops of a trip day by day
  trip add <trip-id> --day N --poi ID...,Append POIs to a day (skips duplicates)
  trip remove <trip-id> --stop ID...,Remove stops
  trip delete <trip-id>,Delete a whole trip
  tour list,Popular expert itineraries (--curated for editor picks)
  tour view <tour-id>,An expert itinerary day by day
  tour copy <tour-id>,Copy an expert itinerary into my trips
  poi search <keyword>,Find places and their ids
  poi view <poi-id>,Address, hours, rating, description of a place
  location search <keyword>,Destination keys for trip create
help[2]:
  Run `chictrip-axi trip view <id>` to continue a trip above
  Run `chictrip-axi <command> --help` for flags, defaults, and examples
```

The command index is the static part a future `SKILL.md` generator
keeps; keep summaries to one clause.

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
  `location search` was added during implementation, see below).
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
  `TravelScheduleDetail/Preview?TravelScheduleId=` (protocol.md, "Shared
  trip previews"), so `trip view` can be developed and tested against
  that fixture before a member token is available. The `Add` and
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
