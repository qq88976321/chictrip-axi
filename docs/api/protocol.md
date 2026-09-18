# chicTrip API protocol notes

Recovered on 2026-09-18 by reading the public web app
(https://www.chictrip.com.tw/, a Vue/Vite SPA) and exercising the API
with the app's own anonymous token. chicTrip publishes no API
documentation; everything here is observed behavior and can change
without notice. The endpoint inventory is in [endpoints.md](endpoints.md).

## Transport

- Base URL: `https://api.chictrip.com.tw/` (Azure Front Door, HTTP/2).
- Paths are `/<Controller>/<Action>`; the billboard endpoints
  (`/GetPoiTopN`, `/GetTopNSecondLocationList`, `/GetPreData`,
  `/GetPoiTopNByTravelStatusAndGps`) sit at the root.
- Reads are `GET` with query-string parameters. Writes are `POST`,
  `PUT`, `DELETE` with form bodies (`multipart/form-data` or
  `application/x-www-form-urlencoded`, a few `application/json`).
- Headers the SPA always sends:

  | Header          | Value                         | Required?                     |
  |-----------------|-------------------------------|-------------------------------|
  | `Authorization` | `Bearer <jwt>`                | yes (missing -> apiStatus 003) |
  | `osType`        | `web`                         | not enforced, send anyway     |
  | `language`      | `zhtw`                        | not enforced; `en`/`ja` did not change the payload |
  | `version`       | app version string, e.g. `3.6.5` | not enforced                |

- Typical latency from Taiwan-adjacent regions: 0.3 to 1.2 s per call;
  `Location/GetAllLocation` (1.1 MB) takes about 4 s.

## Response envelope

Every endpoint answers HTTP 200 with the same JSON envelope, including
on business errors. Only an unknown path yields HTTP 404 (still with
the envelope, `apiStatus` `002`, `message` `404`).

```json
{
  "apiStatus": "001",
  "data": { },
  "message": null,
  "requestId": "40001531-0000-a100-b63f-84710c7967bb",
  "traceId": "81ed756c498d90564d7ba809af667fad"
}
```

Some responses spell the status key `ApiStatus`; the SPA normalizes it.
A client must accept both.

### apiStatus codes (observed)

| Code | Meaning                                              | Seen on                                            |
|------|------------------------------------------------------|----------------------------------------------------|
| 001  | success                                              | everything                                         |
| 002  | business error; `message` explains (zh-TW or English)| `Reject Guest Member`, `A non-empty request body is required`, `404`, POI id format error, missing required parameter (category) |
| 003  | token missing, malformed, or expired                 | no `Authorization`, garbage bearer                 |
| 004  | update-time conflict; `data.updateTime` is current   | `TravelScheduleDetail/VerifyUpdateTime`            |
| 006  | quit collaboration / no access to the schedule       | `TravelScheduleDetail/Get` on someone else's trip  |
| 007, 012, 013, 014 | app-level dialogs in the SPA (014 carries `message`) | not reproduced                       |
| 011  | travel schedule deleted or unknown                   | `ExpertTour/TourV2` with a bad id; `TravelScheduleDetail/Preview` with an unknown or deleted id (`TravelSchedule has been deleted`) |

The SPA treats `003` as "refresh the token and retry once"; on refresh
failure it clears the session.

### The "non-empty request body" trap

`A non-empty request body is required` (apiStatus 002) is what the
server says when a GET is missing one of its required query parameters,
not a hint to send a body (GET with a JSON body returns the same error).
Send every parameter the SPA sends, even if empty:

- `PoiSearch/SearchByKeyword` needs `keyword`, `centerLongitude`,
  `centerLatitude` (0 and 0 are accepted).
- `GetPoiTopN` needs `CountryId`, `CityId`, `AreaId`,
  `PoiClassifationId` (the last three may be empty strings).
- `Poi/Search/Get` needs `categoryId` (else a zh-TW "missing parameter
  [category]" message) plus the center/boundary coordinates.
- `Location/SearchV2` and `TravelSchedule/GetWithDetail` were not
  cracked; `ExpertTour/SearchLocation?keyword=` and
  `ExpertTour/TourV2?travelScheduleId=` cover the same needs.
- A JSON body on a form endpoint gets the same message. Every `PUT`,
  `POST`, and `DELETE` under `TravelSchedule*` and `Poi` binds
  `application/x-www-form-urlencoded` (or multipart); a JSON body is
  "empty" to it. So `002 A non-empty request body is required` never
  means "send JSON" and never names a field: check the encoding first,
  then each required field, then each field that must be non-empty
  (verified 2026-09-19 on `TravelScheduleDetail/Update`, and on
  `TravelScheduleDetail/Add` with an empty `TsdCoverMediaId`).

## Authentication

Login is third-party only (Google, Apple, LINE, Hotai SSO) through
`/Login/ThirdParty`, `/Login/AppleLogin`, `/Login/SsoLogin`; there is no
password login. A session is three values kept by the SPA in
localStorage: `accessToken` (HS256 JWT, issuer `ChicTripApi`),
`refreshToken`, `memberId`. `POST /Token/Refresh` with form fields
`refreshToken` and `memberId` returns fresh copies of all three.

### Anonymous (guest) token

When no session exists the SPA falls back to a guest JWT that is
hardcoded in its bundle (`homeStore.*.js`). Decoded payload:

```
sub: 00000000-0000-0000-0000-000000000001
iss: ChicTripApi
nbf: 2023-05-30   exp: 2037-02-05
```

With it every public read in endpoints.md marked `guest` works.
Member-only endpoints answer `002 Reject Guest Member`, and endpoints
that return "my" data (`TravelSchedule/GetMyAndCollaboration`) return
the guest member's own two demo schedules. The token is public by
construction (served to every anonymous visitor) but it is still a
credential chicTrip could rotate; the CLI must treat it as replaceable
configuration, not as an invariant.

### SEO endpoints

`/Seo/Home`, `/Seo/PoiDetail`, `/Seo/MemberCenter` need an `ApiKey`
header that the SPA derives by AES-decrypting a bundled ciphertext.
Out of scope: the same data is reachable through the regular endpoints.

## Identifiers and vocabularies

- POI ids, travel schedule ids, member ids, category ids: UUIDs.
- Locations form a three-level tree country > city > area with small
  integer ids; a `locationKey` is `"<country>,<city>,<area>"` with `0`
  for "any", e.g. `7,7,0` = Japan > Tokyo. Country 6 = Taiwan, 7 =
  Japan. The full tree is `Location/GetAllLocation` (211 countries, 541
  cities, 3329 areas, 1.1 MB, versioned by
  `Config/GetConfig.data.dataVersion.allLocationVersion`);
  `ExpertTour/SearchLocation?keyword=` is the cheap search.
- POI categories (`PoiClassification/GetMember`, works for guests):
  7 rows with `id`, `name` (zh-TW), `icon`
  (`enterTainment`, `food`, `hotel`, ...), `type`. `categoryType` on a
  POI is the `icon` value.
- Travel schedule detail rows (`tsd`) have `tsdType` `basic` for an
  ordinary stop and `flight` for an airport leg (verified 2026-09-18 on
  `ExpertTour/TourV2`, `TravelScheduleDetail/Get`, and the row
  `TravelScheduleDetail/Add` returns). The category is a separate pair,
  `categoryName` (zh-TW) and `categoryIcon` (`enterTainment`, `food`,
  `moon`, `takeOff`, ...).
- Timestamps are Unix seconds (`publishTime`, `updateTime`,
  `createTime`); dates are `YYYY/MM/DD` strings; times `HH:MM`.

## Shapes of the endpoints the CLI uses

Field lists are trimmed to what the CLI maps; see the fixtures the
exploration saved (scratchpad `fixtures/*.json`) for the full objects.

### POI object (search results, nearby, detail)

Search and nearby return the "list" POI (about 40 keys); `GetPoiById`
returns the "detail" POI (about 65 keys, 163 KB for a popular sight
because of `media`, `advertises`, `poiTickets`).

```
id, name, categoryType, address, longitude, latitude,
ratingCount (average, e.g. 4.5), ratingTotal (review count),
favoriteCount, joinCount (times added to itineraries), placeId (Google),
location{locationCountryId, locationCountryName, locationCityId,
         locationCityName, locationAreaId, locationAreaName},
tags[{name, source}], distance (nearby only), promoteTag, operatingStatus
detail only: description, phoneNumber, url, reviewUrl,
  openTimes[{code, name, descript}], localDescription{name, address,
  description}, poiDescriptions[], media[{id, url, type, source,
  likeCount}], poiTickets[{id, title, price?, url?}], advertises[],
  hotelBooking[], nearByExpertTourLocation{name, fullName, locationKey}
```

`PoiSearch/SearchByKeyword` and `Poi/Search/Get` wrap the list as
`{state, totalCount, hasNextPage, mapPoiDisplayMode, result[], poiCount}`;
`totalCount` was `0` even with 12 results, so count `result` instead.

### Expert tour list (`ExpertTour/PopularRanking`, `ExpertTour/Exclusive`)

```
PopularRanking: {nextPage, expertTourDetailList[{travelScheduleId, name,
  destination, coverUrl, expertMemberId, expertName, expertPhotoUrl,
  top2Emoji[], likeCount, usedCount, tags[], tagRemainingAmount,
  coverTag, coverTagStyle, isNewPublish}]}
Exclusive: [{travelScheduleId, name, coverUrl, expertMemberId,
  expertName, expertPhotoUrl, destination, introduction, tag}]
```

### Expert tour detail (`ExpertTour/TourV2?travelScheduleId=`)

```
overview{travelScheduleId, name, destination, totalDay, startDate,
  endDate, introduction, expertName, expertMemberId, likeCount,
  usedCount, youTubeUrl, publishTime, travelPreferList[], monthList[],
  fullPassingLocationList[{cityName, locationList[]}], link{},
  highlightPoiList[{poiId, poiName, joinCount, ratingCount, cityName,
  areaName}], commentCount, commentList[]}
dayList[{day, date, trafficType, weatherInfo,
  tsdList[{id, name, tsdType, poiId, poiName, day, arrivalTime,
    stayTime, arrivalTrafficType, arrivalTrafficTime, note, isHasNote,
    countryName, cityName, areaName, categoryName, categoryIcon,
    longitude, latitude, flightNumber, terminal, ...}]}]
```

### Rankings (`GetPoiTopN`, `GetTopNSecondLocationList`)

```
GetPoiTopN: {topNFirstLocationInfo[{name, countryId, cityId, areaId,
  isSelected, sort}], topNSecondLocationInfo[same], topNClassifationInfo
  [{id, name, ...}], pois[{sort, poiId, poiName, joinCount, ratingCount,
  imageUrl, isFavorite, countryId, countryName, cityId, cityName,
  areaId, areaName}]}  (30 rows)
GetTopNSecondLocationList?countryId=: [{name, countryId, cityId,
  areaId, isSelected, sort}]  (first row is "any area")
```

### Comments (`PoiComment/GetSummaryAndMyPoiComment`, `PoiComment/GetPoiCommentList`)

```
summary: {scoreSummary{averageScore, starScore, poiCommentTotalCount},
  starSummary{oneStarCount..fiveStarCount}, myPoiComment, myPoiCommentStatus{}}
list?poiId&page: {nextPage, poiCommentList[{id, poiId, memberId,
  memberName, memberPhotoUrl, score, message, messageLanguage,
  likeCount, isLike, updateTime, mediaList}]}
```

### Locations

```
ExpertTour/SearchLocation?keyword=: [{name, fullName, locationKey}]
Location/GetPopularDestination: [{image, locationType, locationId, name,
  zoomLevel, latitude, longitude}]
```

## Trip previews: `GET TravelScheduleDetail/Preview`

`robots.txt` names the sitemap index `https://www.chictrip.com.tw/Sitemap`
(`/sitemap.xml` is the SPA's 404 page); its child urlset
`https://www.chictrip.com.tw/UrlSets/ChicTripTs1` advertises thousands of
shared trips as `/?action=preView&preViewTravelId=<uuid>`. The web app
loads them with
`GET TravelScheduleDetail/Preview?TravelScheduleId=<uuid>` (path
without a leading slash in the bundle; works with the guest token) and
gets `{travelScheduleInfo, dayList}`, the same shape as
`TravelScheduleDetail/Get` returns for the owner. `TravelScheduleDetail/
Get` itself answers `006 Quit collaboration` for a trip the token does
not own or collaborate on. A missing or malformed `TravelScheduleId` is
the usual `002` "non-empty request body" message.

Verified live on 2026-09-18 with the guest token (`trip preview`):

- Preview is gated on nothing but the id. A trip created seconds earlier
  in the test account and never shared answered `001` with its full
  detail to the guest token. "Shared" is a notion of the web UI; any
  trip id is readable by anyone who has it.
- An unknown id, and a trip just deleted with `TravelSchedule/Delete`,
  both answer `011 TravelSchedule has been deleted`.
- `travelScheduleInfo.permission` is the OWNER's value (`Owner` when a
  guest reads someone else's trip), not the caller's; `note` is `""`
  when unset; `viewMode` is `DetailMode`.
- A member token is accepted too and gets the same answer.

`travelScheduleInfo` carries `id, name, startDate, endDate, totalDay,
trafficType, updateTime, permission, viewMode, memberId, coverUrl,
note, collaborationList[], destinationList[]` and more; `dayList[]` is
`{day, date, trafficType, weatherInfo, tsdList[]}` with the `tsd` rows
described under "Expert tour detail" (`id`, `name`, `tsdType`, `poiId`,
`arrivalTime`, `stayTime`, `arrivalTrafficType`, `note`, ...).

## Write flows (from the web app's request builders)

Recovered from `TravelScheduleSettings.*.js`, `GlobalComponents.*.js`,
and `home.*.js` on 2026-09-18, then exercised live with a member
token: create, add, and delete on 2026-09-18; edit, notes, legs,
reorder, copy, and custom places on 2026-09-19 (test account, probe
trip `axi-probe-m4`). Field names are PascalCase or camelCase exactly
as the app sends them; the server binds case-insensitively but the
app's spelling is the safe choice. Every `PUT` below takes
`application/x-www-form-urlencoded`, and the response `data` is the
trip's new `updateTime` as a bare integer unless noted.

### Optimistic locking with updateTime

Every travel schedule carries `updateTime` (Unix seconds). Mutations
send the value the client last saw as `TravelScheduleUpdateTime`
(or `updateTime`); a stale value answers `004 Update time conflict`
with `data.updateTime` set to the current value. The app's recipe:

1. `GET TravelScheduleDetail/VerifyUpdateTime?TravelScheduleId=<id>&travelScheduleUpdateTime=0`
   answers `001` or `004`; either way `data.updateTime` is current.
2. Send the mutation with that value.
3. Every mutation response returns the new time
   (`data.travelScheduleUpdateTime` on `TravelScheduleDetail/Add`,
   `data` itself on `Delete`, `data.updateTime` on `AddV2`); chain it
   into the next call.
4. On `004`, re-read `data.updateTime` and retry once.

### Create a trip: `POST TravelSchedule/AddV2`

`application/x-www-form-urlencoded`, extra header `language: zh-tw`.
Verified live 2026-09-18 (trip created, then deleted):

```
CoverMediaId=<id from TravelSchedule/GetSystemCoverList>   REQUIRED, must be real
Name=<text>
StartDate=YYYY/MM/DD
EndDate=YYYY/MM/DD       (at most 59 days after StartDate)
TotalDay=<inclusive day count>
ViewMode=DetailMode
TravelScheduleUserLabelId=<id from TravelScheduleUserLabel/Get>   REQUIRED, must be real
id=
TrafficType=Custom       (also seen: Transit, Driving, Walk, PublicTransport, Flight)
IsForceUpdateTsdRoute=0
updateTime=0
LocationKey[]=7,7,0      (at least one; axios encodes arrays as name[]=value)
```

Response `data`: the full trip summary (`id, memberId, coverMediaId,
coverUrl, name, startDate, endDate, totalDay, trafficType, createTime,
updateTime, collaborationList, permission, viewMode, note`).

Three fields are validated server side and each failure comes back as
the generic `002 A non-empty request body is required` rather than a
field name: an empty `CoverMediaId`, an empty `TravelScheduleUserLabelId`,
and a missing `LocationKey[]`. The web app never hits them because its
form preselects the first system cover, resolves the label, and refuses
to submit without a destination; a client that sends `CoverMediaId=`
(as the first cut of `chictrip-axi trip create` did) gets the same
message for every encoding and header combination, which is what made
the failure look like a transport problem. `destinationList` may be
omitted; the app sends it only when it has resolved destination objects.

`GET TravelSchedule/GetSystemCoverList` (member) returns
`[{id, value}]` where `value` is the image URL; the app uses the first
entry for a new trip. `GET TravelScheduleUserLabel/Get` (member) returns
`[{id, name, isSystem, sort, travelScheduleCount}]` with four system
labels (planning, travelling, done, unlabeled in zh-TW); the app picks
the entry whose `name` is the zh-TW word for "unlabeled" and `isSystem`
is true, falling back to the first entry.

### Add a POI to a day

1. `GET TravelScheduleDetail/GetAddWhere?poiId=<poi>&travelScheduleId=<trip>&travelScheduleUpdateTime=0`
   returns `dayList[]`, one entry per day, each with `addWhereList[]`
   of insertion slots: `{addWhereId, arrivalTsdName, arrivalTsdLat,
   arrivalTsdLon, departureTsdName, departureTsdLat, departureTsdLon,
   sort, isBestOfDay, isBestOfAll}`. The app preselects `isBestOfAll`,
   then `isBestOfDay`, else the first slot; the last slot of a day
   appends after the current last stop.
2. `POST TravelScheduleDetail/Add` (`multipart/form-data` in the app):

```
TravelScheduleId=<trip>
Day=<1-based day>
PoiId=<poi>
AddWhereId=<slot id>
TravelScheduleUpdateTime=<current>
TsdCoverMediaId=<poi.cover.id or empty>
TsdName=<poi.name>
```

   Response (verified live 2026-09-18): `data.tsdInfo` is the whole new
   `tsd` row, including the `id` that `Delete` takes, and
   `data.travelScheduleUpdateTime` is the trip's new update time.
   `addWhereId` is the literal `start` before the day's first stop and
   `end` after its last one; every other slot carries the id of the
   stop the new one is inserted IN FRONT OF. So a day with three stops
   answers `start`, the SECOND stop's id, the THIRD stop's id, `end`:
   there is no slot named after the first stop, because `start` is
   that slot. A day with no stops answers a single slot named `first`.
   (An earlier revision of this file said the slot carries the id of
   the stop it follows; corrected live on 2026-09-19 by adding a stop
   with the second stop's id as `AddWhereId` and finding it in second
   place.)

Related: `POST TravelScheduleDetail/AddByFavoritePoi` adds from a
favorites playlist; `POST TravelScheduleDetail/Copy` (see "Copy a stop
into another day" below) plus `Delete` is how the app moves a stop
across days, though `Sort` moves one across days on its own.

### Remove a stop: `DELETE TravelScheduleDetail/Delete`

Body (the app sends a form body on DELETE):
`TravelScheduleId, Day, TsdId, TravelScheduleUpdateTime`. Response
`data` is the new update time as a bare number (verified live
2026-09-18).

### Edit a stop: `GET TravelScheduleDetail/GetEditInfo` + `PUT TravelScheduleDetail/Update`

The app opens the edit sheet with

```
GET TravelScheduleDetail/GetEditInfo?travelScheduleId=<trip>&tsdId=<tsd>&travelScheduleUpdateTime=<CURRENT>
```

`travelScheduleUpdateTime` must be the trip's current value: `0`,
which `GetAddWhere` and `VerifyUpdateTime` accept, answers `004` here.
`data`:

```
id, name, sort, address,
categoryId, categoryName, categoryIcon, poiClassificationId,
arrivalTime (computed), stayTime (minutes), departureTime (computed),
isUseCustomArrivalTime, customArrivalTime (HH:MM or null),
isUseCustomDepartureTime, customDepartureTime (HH:MM or null),
categoryList[{id, name, icon, type}]
```

The row keeps the computed `arrivalTime` and the pinned
`customArrivalTime` as separate fields; `isUseCustomArrivalTime` says
which one the app shows (same for departure). `categoryList` has
twelve rows: nine `type: Category` (icons `enterTainment`, `food`,
`shop`, `moon`, `rentCar`, `train`, `plane`, `chargingPoint`, `other`)
and three `type: TsdCategory` (`takeOff`, `transfer`, `landing`).
Their ids are the vocabulary for `PoiClassificationId` below. An
unknown `tsdId` answers `002` with the message `TSD Id not found` and
a null payload (verified live 2026-09-19), not a 001 with no data.

The save is `PUT TravelScheduleDetail/Update`,
`application/x-www-form-urlencoded`, every field present (the app
sends the whole sheet back, edited or not):

```
TsdId=<tsd>
Name=<text>
PoiClassificationId=<id from categoryList>
StayTime=<minutes>
IsUseCustomArrivalTime=0|1
CustomArrivalTime=HH:MM          (empty when 0)
IsUseCustomDepartureTime=0|1
CustomDepartureTime=HH:MM        (empty when 0)
TravelScheduleId=<trip>
travelScheduleUpdateTime=<current>
```

Verified live 2026-09-19 on the probe trip: a stop pinned to a 10:30
arrival with a 90 minute stay and released again with flag `0` plus an
empty time, a stop relabelled `takeOff`, and a hotel row given a
pinned 15:00 arrival and 09:00 departure. A `TsdCategory` id as
`PoiClassificationId` turns the row into `tsdType: flight`, and a
`Category` id turns it back. The row's `flightNumber`, `terminal`, and
flight date fields are never written by the web app: flight details
live in the stop name and in the leg note (`SetFlightRoute`). A JSON
body answers `002 A non-empty request body is required` exactly like a
missing field.

### Stop and trip notes

Two levels; the app has no per-day note.

- Stop: `GET TravelScheduleDetail/GetNote?tsdId=&travelScheduleId=&travelScheduleUpdateTime=`
  answers the note as a bare string in `data` (`""` when unset);
  `PUT TravelScheduleDetail/UpdateNote` with `TravelScheduleId, TsdId,
  Note, TravelScheduleUpdateTime` writes it. An empty `Note` clears it.
- Trip: `GET TravelSchedule/GetNote?id=&updateTime=` and
  `PUT TravelSchedule/UpdateNote` with `id, note, updateTime`.

The `tsd` rows of `TravelScheduleDetail/Get` carry the full `note` and
`isHasNote`, and `travelScheduleInfo.note` is the trip note, so
reading a whole trip needs no `GetNote` at all. A POI stop starts with
the POI's description as its note. `Preview` answers `note: ""` for
the trip header and was observed a few seconds stale right after a
write, so only `Get` (owner) is a read-back oracle. Verified live
2026-09-19.

### Legs: how a stop is reached

A leg belongs to the stop it ARRIVES at. Each `tsd` row carries a
`tsdRouteDetailId`; the first stop of a day has `null` there and no
leg. The row shows the outcome as `arrivalTrafficType` and
`arrivalTrafficTime` (minutes).

```
GET TravelScheduleDetailRoute/GetRouteList?tsdRouteDetailId=<id>&trafficType=<T>&travelScheduleId=<trip>&TravelScheduleUpdateTime=<current>
```

`trafficType` is required (`""` is the generic 002): `Driving`,
`Transit`, `Walking`, `TwoWheeler`, `Custom`, or `Flight`. `data`:

```
tsdRouteDetailId
tsdRouteList[{poiRouteDetailId, distance (m), duration (min), summary, isSelected}]   (google)
tsdRouteTransitList[{poiRouteDetailId, distance, duration, fare{currency, value}, lineList[]}]  (jorudan)
tsdCustomRoute{duration, note, trafficType}
tsdRouteSearchSetting{provider, ...}
```

Both lists are `null`, not `[]`, when the requested mode has no
candidates, and the transit rows carry no `summary` and no
`isSelected`. `tsdCustomRoute` echoes the requested `trafficType` with
a null `duration` when the leg is not of that type.

Writes, all `PUT`, all urlencoded, all answering the new `updateTime`:

- `TravelScheduleDetail/SetRoute`: `TsdRouteDetailId,
  PoiRouteDetailId, TravelScheduleId, travelScheduleUpdateTime` picks
  one row of a list.
- `TravelScheduleDetail/SetCustomRoute`: `TsdRouteDetailId, Duration
  (minutes), Note, TravelScheduleId, travelScheduleUpdateTime` writes
  a free-form leg (`arrivalTrafficType: Custom`).
- `TravelScheduleDetail/SetFlightRoute`: the same fields; the leg
  shows as `Flight` with the note (the probe trip carries
  `BR198 TPE-NRT`).
- `TravelScheduleDetail/SetDefaultRouteAndTsdAllDay`:
  `travelScheduleId, day, trafficType, travelScheduleUpdateTime,
  isForceUpdateTsdRoute 0|1` sets the day's default mode
  (`dayList[].trafficType`); with `0` existing legs keep their type
  and only legs computed afterwards pick the mode up, with `1` every
  leg of the day is recomputed. `data` is
  `{travelScheduleUpdateTime, dayData{day, date, trafficType,
  tsdList}}`, so a client needs no re-read.
- `GET TravelScheduleDetailRoute/GetFormulaTrafficTime?departureLat=&departureLon=&arrivalLat=&arrivalLon=`
  answers `{walkingMinute, drivingMinute, transitMinute}` for two
  points, with no trip involved.

The ownership rule matters for clients: to change how you get TO stop
B, send B's `tsdRouteDetailId`, not A's. Verified live 2026-09-19:
Tokyo Station reached by `Driving` in 21 minutes (SetRoute), a hotel
by a 180 minute `Flight` leg (SetFlightRoute), and a scratch stop by a
25 minute `Custom` leg with a note (SetCustomRoute).

### Reorder a day: `PUT TravelScheduleDetail/Sort`

`TravelScheduleId, MoveOutDay, MoveInDay, MoveTsdId, TsdIdList[],
travelScheduleUpdateTime` (urlencoded). `TsdIdList[]` is the whole
TARGET day in its new order, the moved stop included. Verified live
2026-09-19 within one day, and across days: `MoveOutDay=2,
MoveInDay=4` with the target day's list moved the stop to day 4 and it
kept its id, its note, and its stay, so `Copy` plus `Delete` is not
needed for a move.

Day-level operations the CLI does not use, all verified live
2026-09-19 and all answering the new `updateTime`: `PUT
TravelSchedule/SortDay` (`id, dayList[]` = the current day numbers in
the new order, `updateTime`), `PUT TravelSchedule/UpdateStartDate`
(`Id, StartDate, EndDate, updateTime`; the app's "add a day"), and
`DELETE TravelSchedule/DeleteDay` (`id, DeleteDay, StartDate,
EndDate, TotalDay, UpdateTime`).

### Copy a stop into another day: `POST TravelScheduleDetail/Copy`

`TravelScheduleId, CopyDay, CopyTsdId, StayTime, ArrivalTrafficType,
TravelScheduleUpdateTime` (form). The day field is `CopyDay`; an
earlier revision of this file said `Day`. `data` is `{tsdInfo,
travelScheduleUpdateTime}`: the copy is a new row with a new id and no
leg, its note is the POI's, and `StayTime=45` came back as 60.
Verified live 2026-09-19.

### A place chicTrip does not know: `POST Poi/AddCustomPoiForWeb`

Text fields only: `name, categoryId, longitude, latitude, address,
description`. The web app sends `multipart/form-data` with a media
part, but neither is required: `application/x-www-form-urlencoded`
with no media answered `001` on 2026-09-19, so this endpoint needs no
special transport. `categoryId` comes from `GET
PoiClassification/GetCustomPoiCategory` (member): seven rows
`{id, name, icon}` with icons `enterTainment`, `food`, `shop`, `moon`,
`rentCar`, `train`, `plane`. `PoiClassification/GetAll?page=1` answers
`{page, list[], hasNextPage}` with fifteen rows instead: the same
seven plus `chargingPoint`, `other`, `heart`, `parking`, `pin`, and
three `type: Tag` rows, so it is the wrong list for this form.

`data` is the full POI object with `authority: private` and
`createMode: custom`; `TravelScheduleDetail/Add` accepts its id like
any other and the description becomes the stop's note. Private places
do not appear in `PoiSearch/SearchByKeyword` and there is no delete
endpoint, so every call files a permanent row.

### Best sort (not used): `PreviewBestSortByDayV2` and `SaveBestSortByDayV2`

`GET TravelScheduleDetail/PreviewBestSortByDayV2` (`travelScheduleId,
day, startTsdId, endTsdId, firstArrivalTime,
travelScheduleUpdateTime`) answers a `002` whose message says the stop
count is outside the limit below
`travelScheduleInfo.bestSortTsdLimitCount` (`{minCount: 4, maxCount:
40}`); `PUT TravelScheduleDetail/SaveBestSortByDayV2` is the one JSON
body in the app (`{TravelScheduleId, Day, TravelScheduleUpdateTime,
RecommendResult, ChooseResult, BestSortTsdList[{tsdId, arrivalTime,
arrivalTrafficTime, departureTime, sort, stayTime}]}`) and was not
cracked. Out of scope for the CLI.

### Trip-level edits

- `PUT TravelSchedule/UpdateV3`: AddV2's shape plus `id` and
  `updateTime` (urlencoded, `language: zh-tw`).
- `DELETE TravelSchedule/DeleteDay`: `id, DeleteDay, StartDate,
  EndDate, TotalDay, UpdateTime`.
- `DELETE TravelSchedule/Delete`: `id`. Deletes the whole trip.
  Response `data` is `true`, and deleting a trip that is already gone
  is also `001 true` rather than `011`, so the call is idempotent
  (verified live 2026-09-18).
- `POST ExpertTour/TravelScheduleCopy`: `travelScheduleId` (an expert
  tour id). Copies the tour into the member's own trips; the app only
  checks for `001` and reloads the trip list.

### Reading my trips

- `GET TravelSchedule/GetMyAndCollaboration?updateTime=0&orderByColumn=updatetime&sort=desc`
  lists own and shared trips: `{id, memberId, name, startDate, endDate,
  totalDay, trafficType, createTime, updateTime, permission
  (Owner|Editor|Viewer), viewMode, note, collaborationList[], coverUrl}`.
- `GET TravelScheduleDetail/Get?travelScheduleId=&TravelScheduleUpdateTime=&isMyTravelSchedule=1`
  returns `{travelScheduleInfo, dayList[{day, date, tsdList[]}]}` with
  the same `tsd` rows as `ExpertTour/TourV2`.

### Token refresh: `POST Token/Refresh`

Form fields `refreshToken`, `memberId`. On `001`, `data` holds new
`accessToken`, `refreshToken`, `memberId`; the app stores all three and
replays the failed request with the new bearer.
