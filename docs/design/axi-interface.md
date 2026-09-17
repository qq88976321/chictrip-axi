# AXI interface design: chicTrip commands (milestone 2)

Status: DRAFT for user review, 2026-09-18. Source facts are in
[../api/protocol.md](../api/protocol.md) and
[../api/endpoints.md](../api/endpoints.md). Design rules come from the
`axi` skill (https://axi.md/) and the repo CLAUDE.md. Sample values are
romanized here to keep the doc ASCII; the live API returns zh-TW text.

## Scope

Read-only commands over the endpoints that work with chicTrip's
anonymous token, plus the shared output/error layer every later command
reuses. In scope: POIs (search, view, nearby, comments, rankings),
expert itineraries (list, view), locations (search, ranking areas),
categories, and a live home view. Out of scope (later milestones):
member login and token refresh, itinerary and playlist mutations,
shared-trip preview, `skills/chictrip-axi/SKILL.md` generation with its
`--check` gate, `setup hooks`.

## Command tree

```
chictrip-axi                       home view (identity + live tours + command index)
chictrip-axi poi search <keyword> [--near LAT,LNG] [--limit N]
chictrip-axi poi view <id> [--full]
chictrip-axi poi nearby <id> [--limit N]
chictrip-axi poi comments <id> [--page N]
chictrip-axi poi top --country ID [--city ID] [--area ID] [--category ID|NAME] [--limit N]
chictrip-axi tour list [--curated] [--page N] [--limit N]
chictrip-axi tour view <id> [--day N] [--full]
chictrip-axi location search <keyword>
chictrip-axi location areas --country ID
chictrip-axi category list
```

Global flags, accepted by every command and never reported as unknown:
`--json`, `--fields A,B,C`, `--timeout SECS` (default 30), `--token JWT`,
`--help`, `-V/--version`. Environment: `CHICTRIP_AXI_TOKEN` (same as
`--token`), `CHICTRIP_AXI_BASE_URL` (default
`https://api.chictrip.com.tw/`; exists so tests can point at a local
fixture server and never touch the network).

Nouns are singular (`poi`, `tour`, `location`, `category`); verbs are
`search`, `view`, `list`, `nearby`, `comments`, `top`, `areas`. Ids are
positional; filters are flags. Every list caps rows client-side with
`--limit` (defaults below) and says so in `count`.

## Output contract

- stdout is TOON (spec v4.1, flat shapes only: objects, primitive
  arrays, tabular arrays of uniform flat objects). `--json` prints the
  same document as one line of compact JSON. Nothing else is ever
  written to stdout; progress and debug go to stderr.
- Keys are `snake_case` so they never need quoting. Values are left to
  the encoder's quoting rules (commas, colons, leading `-` get quoted).
- Numbers stay numbers (`rating: 4.5`, `reviews: 36177`); never
  "4.5 (36177 reviews)". Coordinates are two fields `lat`, `lng`.
- Lists: a `count` line first, then the table, then `help`. When the
  API reports paging, `page` and `next_page` appear (omit `next_page`
  when there is none). When `--limit` trimmed rows, `count: 20 of 35`.
- Empty results are definitive: `count: 0`, `pois[0]:`, plus a `help`
  that suggests the next move. Exit 0.
- Long text (`description`, `introduction`, `note`) is truncated to 500
  chars with `... (truncated, N chars total)` appended and the `--full`
  hint in `help`; `--full` prints everything and adds the heavy
  sub-lists (media, tickets, notes). Truncation hints appear only when
  something was actually cut.
- `help[N]` holds complete commands with `<placeholders>` for runtime
  values and carries forward disambiguating flags. Detail views that
  answer the question fully print no `help`.
- `--fields` restricts a list or detail to the named fields, in the
  given order; an unknown name is a usage error that lists the valid
  fields inline.

## Error contract

Errors are TOON on stdout, same channel as data, then a non-zero exit.

```
error: auth_required
message: this command needs a chicTrip member token
help[1]:
  Set CHICTRIP_AXI_TOKEN (or pass --token) to a member access token
```

| code            | exit | when                                                        |
|-----------------|------|-------------------------------------------------------------|
| `usage`         | 2    | clap rejects argv, unknown `--fields` name, bad `--near`; before any network call. `flags[N]` lists the command's valid flags inline so the fix takes one turn |
| `auth_required` | 1    | apiStatus 002 with message `Reject Guest Member`             |
| `auth_invalid`  | 1    | apiStatus 003 (token missing, malformed, expired)            |
| `not_found`     | 1    | HTTP 404, apiStatus 011, id-format messages                  |
| `forbidden`     | 1    | apiStatus 006                                                |
| `conflict`      | 1    | apiStatus 004                                                |
| `api_error`     | 1    | any other non-001 status; `message` carries the upstream text verbatim (it is a sentence, never a body dump) |
| `network`       | 1    | connect/DNS/TLS failure or timeout; `message` names the host and the timeout |
| `internal`      | 1    | response did not parse as the envelope                      |

`requestId` from the envelope is added as `request_id` on API errors so
a bug report can quote it. Stack traces and raw bodies never appear.
`--help` and `--version` always succeed before anything else loads.

## Commands in detail

### Home view (bare invocation)

One network call (`ExpertTour/PopularRanking?page=1&pageSize=5`). If it
fails, the identity and command index still print, `tours` becomes an
`error`-shaped block, and the exit code is 1.

```
bin: ~/.local/bin/chictrip-axi
description: Agent-first CLI for the chicTrip travel API: POIs, expert itineraries, rankings
tours[5]{id,name,destination,likes}:
  daebf5f2-...,Tokyo 7 days 6 nights notes,Japan,4014
  ...
commands[10]{command,summary}:
  poi search <keyword>,Find POIs by keyword (optionally near LAT,LNG)
  poi view <id>,One POI with address, hours, rating, description
  poi nearby <id>,POIs around another POI
  poi comments <id>,Rating summary and reviews of a POI
  poi top --country ID,Most-visited POIs of a country/city/area
  tour list,Popular expert itineraries (--curated for editor picks)
  tour view <id>,An itinerary day by day
  location search <keyword>,Resolve a place name to location ids
  location areas --country ID,Cities/areas usable as ranking filters
  category list,POI categories and their ids
help[2]:
  Run `chictrip-axi tour view <id>` to read one of the tours above
  Run `chictrip-axi <command> --help` for flags, defaults, and examples
```

The command index is the static part a future `SKILL.md` generator
strips live state from; keep the summaries to one clause each.

### poi search

`PoiSearch/SearchByKeyword?keyword=&centerLongitude=&centerLatitude=`
(0,0 when `--near` is absent; the API ranks by distance when given).
Default `--limit 20`.

```
count: 12 of 12
pois[12]{id,name,category,rating,city,area}:
  edd5509c-...,Azumabashi pier,enterTainment,4,Tokyo,Sumida
  ...
help[2]:
  Run `chictrip-axi poi view <id>` for hours, address, and description
  Run `chictrip-axi poi nearby <id>` for places around one result
```

Extra fields via `--fields`: `reviews` (ratingTotal), `favorites`,
`visits` (joinCount), `lat`, `lng`, `address`, `place_id`, `country`.
`hasNextPage` maps to `more: true` when set (no page parameter is known,
so the hint suggests `--near` or a narrower keyword).

### poi view

`Poi/GetPoiById?id=`. Detail view, no `help` unless truncated.

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
`tickets[N]{title,price,url}` and prints `description` whole. Null
fields are omitted rather than printed as `null`.

### poi nearby

`Poi/GetPoisByPoiNearBy?poiId=`. Same table as search plus
`distance_m` when the API fills `distance`. Default `--limit 12` (the
API returns 12).

### poi comments

Two calls: `PoiComment/GetSummaryAndMyPoiComment?poiId=` and
`PoiComment/GetPoiCommentList?poiId=&page=`.

```
summary:
  average: 5
  total: 3
  stars[5]: 0,0,0,0,3
count: 3
page: 1
comments[3]{author,score,message,likes,date}:
  Editor Ruru,5,,0,2026-08-15
  ...
help[1]:
  Run `chictrip-axi poi comments <id> --page 2` for more   (only when next_page > 0)
```

`stars` is the 1-to-5 star histogram in order. `date` is `updateTime`
rendered as `YYYY-MM-DD` UTC.

### poi top

`GetPoiTopN?CountryId=&CityId=&AreaId=&PoiClassifationId=` (all four
sent, empty when unset). `--category` accepts a category UUID or one of
the names/icons from `category list` (resolved through
`PoiClassification/GetMember` only when a non-UUID is given). Default
`--limit 30` (the API returns 30).

```
count: 30 of 30
filters:
  country: 7
  city: 7
  category: enterTainment
pois[30]{rank,id,name,rating,visits,city,area}:
  1,8a48a94c-...,Senso-ji Kaminarimon,4.5,220643,Tokyo,
  ...
help[2]:
  Run `chictrip-axi poi top --country 7 --city <cityId> --area <areaId>` to narrow (ids: `chictrip-axi location areas --country 7`)
  Run `chictrip-axi poi view <id>` for details
```

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
  ...
help[2]:
  Run `chictrip-axi tour view <id>` to read an itinerary day by day
  Run `chictrip-axi tour list --page 2` for the next page
```

`--fields` extras: `tags` (joined with `|`) and, for `--curated`,
`introduction`. Day counts are not in the list payload; `tour view` has
them.

### tour view

`ExpertTour/TourV2?travelScheduleId=`. The stops table is the content;
`--day N` keeps one day. Default prints every day.

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
  1,2,18:30,90,Shinjuku Station,poi,Tokyo,....
  ...
help[2]:
  Run `chictrip-axi poi view <poi_id>` for a stop
  Run `chictrip-axi tour view 8c9b156f-... --full` for notes and traffic per stop
```

`--full` adds `note`, `traffic` (arrivalTrafficType), `traffic_min`,
`flight` (flightNumber) columns, the whole `introduction`, and a second
table `highlights[N]{poi_id,name,rating,visits}` built from
`overview.highlightPoiList`.

### location search

`ExpertTour/SearchLocation?keyword=`.

```
count: 8
locations[8]{name,full_name,country,city,area}:
  Tokyo,Japan/Tokyo,7,7,0
  Shinjuku,Japan/Tokyo/Shinjuku,7,7,6
help[1]:
  Run `chictrip-axi poi top --country <country> --city <city> [--area <area>]` to rank POIs there
```

The three id columns are the split `locationKey`; the joined key is
available as `--fields key`.

### location areas

`GetTopNSecondLocationList?countryId=`.

```
count: 12
areas[12]{name,city,area}:
  any area,0,0
  Tokyo,7,0
  Kyoto,9,45
help[1]:
  Run `chictrip-axi poi top --country 7 --city <city> --area <area>`
```

### category list

`PoiClassification/GetMember`.

```
count: 7
categories[7]{id,name,icon}:
  9b471f78-...,Sights,enterTainment
  9449daa2-...,Food,food
help[1]:
  Run `chictrip-axi poi top --country 7 --category <icon>`
```

## Module layout (src/)

```
main.rs        parse argv (catch clap errors -> usage error on stdout), run, map exit code
cli.rs         clap tree (Cli, Noun, Verb enums), global flags, home view assembly
api/mod.rs     Client { base_url, token, timeout }: get(path, &[(k, v)]) -> Envelope<Value>
               Envelope { api_status, data, message, request_id } (accepts ApiStatus too)
               status -> AxiError mapping lives here, once
api/types.rs   serde structs for the endpoints used (deny nothing unknown; skip unknown fields)
output.rs      Document builder: count/page lines, tables (Vec<Row>), truncate(text, 500),
               help lines; render_toon(&Document) / render_json(&Document); --fields filter
error.rs       AxiError { code, message, help, request_id } + exit_code(); Usage stays exit 2
commands/      poi.rs, tour.rs, location.rs, category.rs: fetch -> map to Document; no
               formatting or error text inside (shared layer only)
```

Tests: unit tests per module on saved fixtures (trimmed copies under
`tests/fixtures/`), plus one integration test that starts a
`std::net::TcpListener` fixture server and runs the binary with
`CHICTRIP_AXI_BASE_URL` pointing at it (exit codes, TOON snapshots,
error shapes, unknown flag -> exit 2 with `flags[]`). No test touches
the network or the user's HOME.

## Decisions that need the user (rule N12)

### D1. Crates to add

Four crates would be added to Cargo.toml (today: clap, anyhow,
thiserror). All are MIT/Apache, MSRV <= 1.85, and pass `cargo audit`
as of 2026-09-18.

| Need          | Recommended                                           | Alternatives considered |
|---------------|-------------------------------------------------------|-------------------------|
| HTTP + TLS    | `ureq = { version = "3", features = ["json"] }` (sync, rustls+ring, gzip; rust-version 1.85; the release workflow builds musl with `cross`, so ring's C code cross-compiles) | `reqwest` blocking (pulls tokio, 3x compile time); `minreq`/`attohttpc` (smaller communities) |
| JSON          | `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"` | none realistic |
| TOON encoder  | `toon-format = { version = "0.5", default-features = false }` (official Rust impl, spec v3.0; our flat shapes encode identically under v4.1; default `cli` feature would drag ratatui, so it is turned off; deps serde, serde_json, indexmap, thiserror) | `etoon 0.8` (tracks v4.1, encoder only, but 1.7k downloads, single maintainer, depends on sonic-rs); hand-rolled 150-line encoder for our subset (zero deps, but we own spec conformance) |

### D2. Anonymous token handling

The CLI needs a bearer token for every call. Options:

- A (recommended): embed chicTrip's public guest JWT as the default,
  overridable by `--token` / `CHICTRIP_AXI_TOKEN`. Zero-config for
  agents; identical to what every anonymous browser sends. The token is
  already public in the SPA bundle, but it is a JWT literal in a public
  repo: secret scanners may flag it, and chicTrip can rotate it (then a
  patch release updates the constant; `auth_invalid` tells the agent to
  set a token meanwhile).
- B: no default; require `CHICTRIP_AXI_TOKEN`. Safe, but every fresh
  agent session fails until a human copies a token out of a browser.
- C: fetch the guest token at runtime from the SPA bundle and cache it
  under `$XDG_CACHE_HOME`. Self-healing but fragile (bundle hashes
  change) and adds a second host and a cache dir to reason about.

### D3. Command scope for this milestone

The ten commands above (recommended), or a smaller first cut (`poi
search`, `poi view`, `tour list`, `tour view` plus the home view) with
the rest as a follow-up.

## Implementation notes for the executing agent

- Read `.agents/skills/axi/SKILL.md` and the TOON spec (a copy of
  SPEC.md v4.1 is in the session scratchpad; upstream:
  https://github.com/toon-format/spec/blob/main/SPEC.md) before writing
  the output layer.
- Send `osType: web` and `language: zhtw` on every request; accept
  `apiStatus` and `ApiStatus`.
- Always send every required query parameter (see protocol.md, "The
  non-empty request body trap").
- Validate `--near` (`LAT,LNG` floats), `--limit` (1..=200), `--page`
  (>= 1) before the client is even constructed.
- Version fast path: clap handles `-V/--version` before any client or
  network work; keep `main` free of eager network calls.
- Follow repo rules: ASCII-only sources and comments, `just gate` green
  before each commit, conventional commits with scopes `feat(api)`,
  `feat(output)`, `feat(poi)`, `feat(tour)`, `feat(location)`,
  `feat(cli)`, `test`, `docs`; never push.
