# chicTrip API endpoint map (158 endpoints)

Recovered on 2026-09-18 from the public SPA bundles at
https://www.chictrip.com.tw/assets/*.js (Vite build). Base URL
`https://api.chictrip.com.tw/`. Method = the axios helper the SPA uses.
Auth column: `guest` = works with the anonymous guest JWT, `member` =
replies `002 Reject Guest Member`, `token` = works for guest but returns
the guest member's own data, `apikey` = needs the SEO ApiKey header, `?` =
not verified. Params are query-string names for GET, form fields
otherwise. Rows marked `(bundle)` in the Params column were read from the web app's request builders and not exercised; rows marked `(live)` were exercised with the guest token on 2026-09-18 or with a member token on 2026-09-18 and 2026-09-19 (test account).

## Advertise (1)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/Advertise/GetActiveAdvertiseByPosition` |  |  |  |

## Config (7)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/Config/GetConfig` | guest | - | site config, dataVersion.allLocationVersion |
| GET | `/Config/GetHotelBookingPage` |  |  |  |
| GET | `/Config/GetInsurancePage` |  |  |  |
| GET | `/Config/GetShoppingPage` |  |  |  |
| GET | `/Config/GetThemeTourPage` |  |  |  |
| GET | `/Config/GetTrafficPage` |  |  |  |
| GET | `/Config/GetTravelExperiencePage` |  |  |  |

## DeepLink (9)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/DeepLink/CreateCollaborationDeepLink` |  |  |  |
| POST | `/DeepLink/CreateExpertTourDeepLink` |  |  |  |
| POST | `/DeepLink/CreateInAppWebViewDeepLink` |  |  |  |
| POST | `/DeepLink/CreateMainDeepLink` |  |  |  |
| POST | `/DeepLink/CreatePoiInfoDeepLink` |  |  |  |
| POST | `/DeepLink/CreatePreViewDeepLink` |  |  |  |
| POST | `/DeepLink/CreateProfileDeepLink` |  |  |  |
| POST | `/DeepLink/CreateShareExpertPlaylistDeepLink` |  |  |  |
| POST | `/DeepLink/CreateSharePlaylistDeepLink` |  |  |  |

## ExpertTour (11)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/ExpertTour/Exclusive` | guest | - | 8 curated tours |
| POST | `/ExpertTour/Highlight` |  |  |  |
| GET | `/ExpertTour/NineGridLocation` | guest | gridCount | nineGridList + lastList |
| GET | `/ExpertTour/PopularRanking` | guest | page, pageSize | nextPage + expertTourDetailList |
| POST | `/ExpertTour/SaveLocation` |  |  |  |
| GET | `/ExpertTour/SearchLocation` | guest | keyword | [{name, fullName, locationKey}] |
| POST | `/ExpertTour/TourGuide` |  |  |  |
| POST | `/ExpertTour/TourGuideOption` |  |  |  |
| GET | `/ExpertTour/TourV2` | guest | travelScheduleId | overview + dayList[].tsdList[]; 57 KB |
| POST | `/ExpertTour/TravelScheduleCopy` | member | travelScheduleId (bundle) | copies an expert tour into my trips |
| POST | `/ExpertTour/TravelScheduleLike` |  |  |  |

## (root: Billboard) (4)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/GetPoiTopN` | guest | CountryId, CityId, AreaId, PoiClassifationId (all present, may be empty) | topNFirstLocationInfo, topNSecondLocationInfo, topNClassifationInfo, pois |
| GET | `/GetPoiTopNByTravelStatusAndGps` |  |  |  |
| GET | `/GetPreData` | guest | date | home theme config |
| GET | `/GetTopNSecondLocationList` | guest | countryId | cities/areas for the ranking filter |

## HotaiMember (3)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/HotaiMember/ConfirmCenterUnbindMember` |  |  |  |
| GET | `/HotaiMember/GetModifyRedirectUri` |  |  |  |
| GET | `/HotaiMember/GetWebHashOneId` |  |  |  |

## Location (5)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/Location/GetAllLocation` | guest | - | 1.1 MB: countries 211, cities 541, areas 3329, searchLocationList 4081 |
| GET | `/Location/GetPopularDestination` | guest | - | 4 items: name, locationType, locationId, lat, lng |
| GET | `/Location/GetRecommend` | guest | - | promo banner, low value |
| GET | `/Location/SearchV2` | ? | unknown (rejects keyword) | replies 'A non-empty request body is required' |
| GET | `/Location/TravelScheduleDestinationNineGrid` | guest | - | 9 destinations with locationKey |

## Log (1)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/Log/AddTravelScheduleAccessLogAsync` |  |  |  |

## Login (3)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/Login/AppleLogin` |  |  |  |
| POST | `/Login/SsoLogin` |  |  |  |
| POST | `/Login/ThirdParty` |  |  |  |

## Media (3)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| PUT | `/Media/UpdateMemberPhotoBase64` |  |  |  |
| PUT | `/Media/UpdateTravelScheduleCoverBase64` |  |  |  |
| POST | `/Media/UploadTravelScheduleCoverBase64` |  |  |  |

## MemberCenter (18)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/MemberCenter/BindHotai` |  |  |  |
| DELETE | `/MemberCenter/DeleteMedia` |  |  |  |
| DELETE | `/MemberCenter/DeleteMember` |  |  |  |
| POST | `/MemberCenter/FollowMember` |  |  |  |
| GET | `/MemberCenter/GetFilmListPaging` |  |  |  |
| GET | `/MemberCenter/GetLikeMediaListPaging` |  |  |  |
| GET | `/MemberCenter/GetMemberInfoV2` |  |  |  |
| GET | `/MemberCenter/GetMemberSettings` |  |  |  |
| GET | `/MemberCenter/GetPhotoListPaging` |  |  |  |
| GET | `/MemberCenter/GetUpdateMemberInfo` |  |  |  |
| DELETE | `/MemberCenter/UnFollowMember` |  |  |  |
| PUT | `/MemberCenter/UpdateMedia` |  |  |  |
| PUT | `/MemberCenter/UpdateMember` |  |  |  |
| PUT | `/MemberCenter/UpdateMemberIntroduction` |  |  |  |
| PUT | `/MemberCenter/UpdateMemberNickname` |  |  |  |
| PUT | `/MemberCenter/UpdateMemberPersonalInfo` |  |  |  |
| PUT | `/MemberCenter/UpdateMemberSocialMedia` |  |  |  |
| PUT | `/MemberCenter/UpdateTravelScheduleSort` |  |  |  |

## Playlist (9)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/Playlist/AddPlaylist` |  |  |  |
| POST | `/Playlist/CopyPoisToPlaylists` |  |  |  |
| GET | `/Playlist/GetPlaylist` | member | playlistId |  |
| GET | `/Playlist/GetPlaylists` | member | - | guest -> 002 |
| DELETE | `/Playlist/RemovePlaylist` |  |  |  |
| DELETE | `/Playlist/RemovePlaylistPois` |  |  |  |
| POST | `/Playlist/SetPoisToPlaylists` |  |  |  |
| POST | `/Playlist/TravelImport` |  |  |  |
| PUT | `/Playlist/UpdatePlaylist` |  |  |  |

## Poi (6)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/Poi/AddCustomPoiForWeb` | member | name, categoryId, longitude, latitude, address, description (urlencoded, no media; live) | files a private place: returns the full POI with authority=private and createMode=custom; not searchable afterwards and there is no delete endpoint; the web app sends multipart but urlencoded is accepted |
| POST | `/Poi/Favorite/Post` |  |  |  |
| GET | `/Poi/GetPoiById` | guest | id | full POI, 163 KB (media, advertises, poiTickets) |
| GET | `/Poi/GetPoisByPoiNearBy` | guest | poiId | 12 full POIs, 248 KB |
| POST | `/Poi/MediaLike/Post` |  |  |  |
| GET | `/Poi/Search/Get` | guest | countryId, cityId, areaId, searchMode=location, categoryId (required), centerLongitude, centerLatitude, boundaryLongitude, boundaryLatitude, displayMode=poi | {state,totalCount,hasNextPage,result[],poiCount} |

## PoiClassification (4)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/PoiClassification/GetAll` | member | page (live) | {page, list[], hasNextPage}; 15 rows on page 1: the 7 custom-POI icons plus chargingPoint, other, heart, parking, pin and three type=Tag rows, so not the vocabulary AddCustomPoiForWeb wants; guest -> 002 Reject Guest Member |
| GET | `/PoiClassification/GetCustomPoiCategory` | member | - (live) | 7 rows {id, name, icon}: enterTainment, food, shop, moon, rentCar, train, plane; the categoryId vocabulary for AddCustomPoiForWeb; guest -> 002 |
| GET | `/PoiClassification/GetMember` | guest | - | 7 categories: id, name, icon, type |
| PUT | `/PoiClassification/UpdateMember` |  |  |  |

## PoiComment (11)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/PoiComment/AddPoiComment` |  |  |  |
| POST | `/PoiComment/AddPoiCommentV2` |  |  |  |
| DELETE | `/PoiComment/CancelLikePoiComment` |  |  |  |
| DELETE | `/PoiComment/DeletePoiComment` |  |  |  |
| GET | `/PoiComment/GetPoiCommentById` |  |  |  |
| GET | `/PoiComment/GetPoiCommentList` | guest | poiId, page | nextPage + poiCommentList |
| GET | `/PoiComment/GetSummaryAndMyPoiComment` | guest | poiId | scoreSummary + starSummary |
| POST | `/PoiComment/LikePoiComment` |  |  |  |
| PUT | `/PoiComment/ReportPoiComment` |  |  |  |
| PUT | `/PoiComment/UpdatePoiComment` |  |  |  |
| PUT | `/PoiComment/UpdatePoiCommentV2` |  |  |  |

## PoiSearch (1)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/PoiSearch/SearchByKeyword` | guest | keyword, centerLongitude, centerLatitude (all required; 0,0 allowed) | {state,totalCount,hasNextPage,result[],poiCount}; totalCount is 0 even with results |

## Product (2)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/Product/PoiDetail` |  |  |  |
| GET | `/Product/PoiDetail/NearBy` |  |  |  |

## Share (1)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/Share/GetCollaborationLink` |  |  |  |

## Token (1)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/Token/Refresh` | - | refreshToken, memberId (form) | returns accessToken, refreshToken, memberId |

## TravelSchedule (14)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/TravelSchedule/AddV2` | member | CoverMediaId, Name, StartDate, EndDate, TotalDay, ViewMode=DetailMode, TravelScheduleUserLabelId, id="", TrafficType, IsForceUpdateTsdRoute=0, updateTime=0, LocationKey[] (urlencoded; live) | returns {id, name, updateTime, permission}; an empty CoverMediaId, label id, or LocationKey[] -> the generic 002 |
| POST | `/TravelSchedule/Copy` |  |  |  |
| DELETE | `/TravelSchedule/Delete` | member | id (form; live) | whole trip; deleting one that is already gone also answers 001 true; the bundle writes the path without a leading slash |
| DELETE | `/TravelSchedule/DeleteDay` | member | id, DeleteDay, StartDate, EndDate, TotalDay, UpdateTime (form; live) | removes one day; returns the new updateTime; not used by the CLI |
| PUT | `/TravelSchedule/SortDay` | member | id, dayList[] (the current day numbers in the new order), updateTime (urlencoded; live) | reorders days; returns the new updateTime; path has no leading slash in the bundle; not used by the CLI |
| PUT | `/TravelSchedule/UpdateStartDate` | member | Id, StartDate, EndDate, updateTime (urlencoded; live) | moves or extends the date span; the app uses it to add a day; returns the new updateTime; no leading slash in the bundle; not used by the CLI |
| GET | `/TravelSchedule/Get` |  |  |  |
| GET | `/TravelSchedule/GetCollaboration` |  |  |  |
| GET | `/TravelSchedule/GetCollaborationWithDetail` |  |  |  |
| GET | `/TravelSchedule/GetMyAndCollaboration` | token | updateTime=0, orderByColumn=updatetime, sort=desc | my trips + collaborations (live for guest demo data) |
| GET | `/TravelSchedule/GetMyAndCollaborationWithTsdCount` | token | - | guest sees 2 demo schedules |
| GET | `/TravelSchedule/GetNote` | member | id, updateTime (live) | data is the trip note as a bare string ("" when unset); TravelScheduleDetail/Get carries the same text, so the CLI does not call this |
| GET | `/TravelSchedule/GetSystemCoverList` | member | - | guest -> 002 |
| GET | `/TravelSchedule/GetUpdateTravelScheduleInfo` |  |  |  |
| GET | `/TravelSchedule/GetWithDetail` |  |  |  |
| PUT | `/TravelSchedule/UpdateNote` | member | id, note, updateTime (urlencoded; live) | returns the new updateTime; an empty note clears it; Preview keeps answering note "" |
| PUT | `/TravelSchedule/UpdateV3` | member | same shape as AddV2 plus id and updateTime (urlencoded; bundle) |  |

## TravelScheduleCollaboration (7)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/TravelScheduleCollaboration/Add` |  |  |  |
| POST | `/TravelScheduleCollaboration/AddWithToken` |  |  |  |
| DELETE | `/TravelScheduleCollaboration/DeleteByOwner` |  |  |  |
| GET | `/TravelScheduleCollaboration/Get` |  |  |  |
| GET | `/TravelScheduleCollaboration/GetWithToken` |  |  |  |
| DELETE | `/TravelScheduleCollaboration/Quit` |  |  |  |
| PUT | `/TravelScheduleCollaboration/UpdatePermissionByOwner` |  |  |  |

## TravelScheduleDetail (22)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/TravelScheduleDetail/Add` | member | TravelScheduleId, Day, PoiId, AddWhereId, TravelScheduleUpdateTime, TsdCoverMediaId, TsdName (urlencoded; live) | returns {tsdInfo, travelScheduleUpdateTime}; AddWhereId is start, end, or the id of the stop to insert IN FRONT OF (an empty day has one slot named first); an empty TsdCoverMediaId -> the generic 002 |
| POST | `/TravelScheduleDetail/AddByFavoritePoi` |  |  |  |
| POST | `/TravelScheduleDetail/Copy` | member | TravelScheduleId, CopyDay, CopyTsdId, StayTime, ArrivalTrafficType, TravelScheduleUpdateTime (form; live) | copies a stop into day CopyDay: {tsdInfo, travelScheduleUpdateTime}; new tsd id, no leg; StayTime 45 came back 60; Sort moves a stop across days without copying |
| DELETE | `/TravelScheduleDetail/Delete` | member | TravelScheduleId, Day, TsdId, TravelScheduleUpdateTime (form body on DELETE; live) | returns the new updateTime |
| GET | `/TravelScheduleDetail/Get` | token | travelScheduleId, TravelScheduleUpdateTime, isMyTravelSchedule | {travelScheduleInfo, dayList[].tsdList[]}; guest on a shared trip -> 006 (live) |
| GET | `/TravelScheduleDetail/Preview` | guest | TravelScheduleId | preview of ANY trip by id, shared or not (live, an unshared own trip included): {travelScheduleInfo, dayList}; unknown or deleted id -> 011 `TravelSchedule has been deleted`; missing or malformed id -> 002 non-empty body; no leading slash in the bundle |
| GET | `/TravelScheduleDetail/GetAddWhere` | member | poiId, travelScheduleId, travelScheduleUpdateTime=0 (bundle) | dayList[].addWhereList[] insertion slots: addWhereId, arrival/departure tsd names, isBestOfDay, isBestOfAll |
| GET | `/TravelScheduleDetail/GetAddWhereBestAll` |  |  |  |
| GET | `/TravelScheduleDetail/GetDetail` |  |  |  |
| GET | `/TravelScheduleDetail/GetEditInfo` | member | travelScheduleId, tsdId, travelScheduleUpdateTime (must be CURRENT, 0 -> 004) (live) | the edit sheet: {id, name, address, categoryIcon, poiClassificationId, arrivalTime, stayTime, departureTime, isUseCustomArrivalTime, customArrivalTime, isUseCustomDepartureTime, customDepartureTime, categoryList[]}; categoryList = 9 Category + 3 TsdCategory rows; an unknown tsdId -> 002 "TSD Id not found" with a null payload |
| GET | `/TravelScheduleDetail/GetNote` | member | tsdId, travelScheduleId, travelScheduleUpdateTime (live) | data is the stop note as a bare string; the tsd rows of Get carry it too |
| GET | `/TravelScheduleDetail/GetPreviewSettingByDay` |  |  |  |
| GET | `/TravelScheduleDetail/PreviewBestSortByDayV2` | member | travelScheduleId, day, startTsdId, endTsdId, firstArrivalTime, travelScheduleUpdateTime (bundle; live -> 002 below 4 stops) | needs 4 to 40 stops in the day (travelScheduleInfo.bestSortTsdLimitCount); not used by the CLI |
| PUT | `/TravelScheduleDetail/SaveBestSortByDayV2` | member | JSON {TravelScheduleId, Day, TravelScheduleUpdateTime, RecommendResult, ChooseResult, BestSortTsdList[]} (bundle; not cracked) | the one JSON body in the app; same 4 to 40 limit; not used by the CLI |
| PUT | `/TravelScheduleDetail/SetCover` |  |  |  |
| PUT | `/TravelScheduleDetail/SetCustomRoute` | member | TsdRouteDetailId, Duration (min), Note, TravelScheduleId, travelScheduleUpdateTime (urlencoded; live) | free-form leg INTO the stop that owns TsdRouteDetailId (arrivalTrafficType Custom); returns the new updateTime |
| PUT | `/TravelScheduleDetail/SetDefaultRouteAndTsdAllDay` | member | travelScheduleId, day, trafficType, travelScheduleUpdateTime, isForceUpdateTsdRoute 0/1 (urlencoded; live) | a day's default mode; answers {travelScheduleUpdateTime, dayData}; 0 keeps existing legs, 1 recomputes every leg of the day |
| PUT | `/TravelScheduleDetail/SetFlightRoute` | member | TsdRouteDetailId, Duration (min), Note, TravelScheduleId, travelScheduleUpdateTime (urlencoded; live) | the leg shows as Flight with the note, e.g. "BR198 TPE-NRT"; returns the new updateTime |
| PUT | `/TravelScheduleDetail/SetRoute` | member | TsdRouteDetailId, PoiRouteDetailId, TravelScheduleId, travelScheduleUpdateTime (urlencoded; live) | picks one row of GetRouteList; returns the new updateTime |
| PUT | `/TravelScheduleDetail/Sort` | member | TravelScheduleId, MoveOutDay, MoveInDay, MoveTsdId, TsdIdList[], travelScheduleUpdateTime (urlencoded; live) | TsdIdList[] is the whole TARGET day in its new order; MoveOutDay != MoveInDay moves the stop across days and it keeps its id, note and stay |
| PUT | `/TravelScheduleDetail/Update` | member | TsdId, Name, PoiClassificationId, StayTime, IsUseCustomArrivalTime 0/1, CustomArrivalTime HH:MM, IsUseCustomDepartureTime 0/1, CustomDepartureTime HH:MM, TravelScheduleId, travelScheduleUpdateTime (urlencoded; live) | send the whole GetEditInfo sheet back; a TsdCategory id makes tsdType=flight; an empty custom time with its flag at 0 is accepted; a JSON body -> the generic 002 |
| PUT | `/TravelScheduleDetail/UpdateNote` | member | TravelScheduleId, TsdId, Note, TravelScheduleUpdateTime (urlencoded; live) | returns the new updateTime; an empty Note clears it |
| GET | `/TravelScheduleDetail/VerifyUpdateTime` | guest | TravelScheduleId, travelScheduleUpdateTime | 001 or 004; data.updateTime is the current value (live) |

## TravelScheduleDetailRoute (3)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/TravelScheduleDetailRoute/GetFormulaTrafficTime` | member | departureLat, departureLon, arrivalLat, arrivalLon (live) | {walkingMinute, drivingMinute, transitMinute}; no trip involved; not used by the CLI |
| GET | `/TravelScheduleDetailRoute/GetRouteDetail` | member | poiRouteDetailId, travelScheduleId, TravelScheduleUpdateTime (bundle) | 002 "PoiRouteDetail no found" for a tsdRouteDetailId; takes a poiRouteDetailId from GetRouteList; not used by the CLI |
| GET | `/TravelScheduleDetailRoute/GetRouteList` | member | tsdRouteDetailId, trafficType (Driving/Transit/Walking/TwoWheeler/Custom/Flight; required, "" -> 002), travelScheduleId, TravelScheduleUpdateTime (live) | {tsdRouteList[{poiRouteDetailId, distance m, duration min, summary, isSelected}], tsdRouteTransitList[.. fare{currency,value}, lineList[]], tsdCustomRoute{duration, note, trafficType}, tsdRouteSearchSetting}; both lists are null rather than [] when empty |

## TravelScheduleShare (3)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/TravelScheduleShare/GetLink` |  |  |  |
| GET | `/TravelScheduleShare/GetText` |  |  |  |
| POST | `/TravelScheduleShare/SendEmail` |  |  |  |

## TravelScheduleUserLabel (5)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| POST | `/TravelScheduleUserLabel/Add` |  |  |  |
| DELETE | `/TravelScheduleUserLabel/Delete` |  |  |  |
| GET | `/TravelScheduleUserLabel/Get` | member | - | labels; default = name "unlabeled" (zh-TW) with isSystem=true |
| PUT | `/TravelScheduleUserLabel/Sort` |  |  |  |
| PUT | `/TravelScheduleUserLabel/Update` |  |  |  |
