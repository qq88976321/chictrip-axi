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
and `home.*.js`; not exercised live in the exploration session (no
member token). Field names are PascalCase or camelCase exactly as the
app sends them; the server appears to bind case-insensitively but the
app's spelling is the safe choice.

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
   `end` after its last one; the slots in between carry the id of the
   stop they follow. A day with no stops has both `start` and `end`.

Related: `POST TravelScheduleDetail/AddByFavoritePoi` adds from a
favorites playlist; `POST TravelScheduleDetail/Copy`
(`{TravelScheduleId, Day, CopyTsdId, StayTime, ArrivalTrafficType,
TravelScheduleUpdateTime}`) plus `Delete` is how the app moves a stop
across days.

### Remove a stop: `DELETE TravelScheduleDetail/Delete`

Body (the app sends a form body on DELETE):
`TravelScheduleId, Day, TsdId, TravelScheduleUpdateTime`. Response
`data` is the new update time as a bare number (verified live
2026-09-18).

### Reorder a day: `PUT TravelScheduleDetail/Sort`

`TravelScheduleId, MoveOutDay, MoveInDay, MoveTsdId, TsdIdList[]
(the day's tsd ids in the new order), travelScheduleUpdateTime`.

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
