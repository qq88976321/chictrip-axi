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
| 011  | travel schedule deleted or unknown                   | `ExpertTour/TourV2` with a bad id                  |

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
- Travel schedule detail rows (`tsd`) have `tsdType` `poi` or `flight`.
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

## Public itineraries that are NOT reachable anonymously

The sitemap advertises thousands of shared trips as
`/?action=preView&preViewTravelId=<uuid>`. Loading one goes through
`TravelScheduleDetail/VerifyUpdateTime` (answers `004` with the current
`updateTime`) and then `TravelScheduleDetail/Get?travelScheduleId=&
TravelScheduleUpdateTime=&isMyTravelSchedule=0`, which for the guest
token answers `006 Quit collaboration`. The web app evidently exchanges
the share link for collaboration access first (`TravelScheduleCollaboration/
GetWithToken`, `AddWithToken`). Left for a later milestone.
