# chicTrip API endpoint map

Recovered on 2026-09-18 from the public SPA bundles at
https://www.chictrip.com.tw/assets/*.js (Vite build). Base URL
`https://api.chictrip.com.tw/`. Method = the axios helper the SPA uses.
Auth column: `guest` = works with the anonymous guest JWT, `member` =
replies `002 Reject Guest Member`, `token` = works for guest but returns
the guest member's own data, `apikey` = needs the SEO ApiKey header, `?` =
not verified. Params are query-string names for GET, form fields
otherwise. Only rows marked in the Auth column were exercised live.

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
| POST | `/ExpertTour/TravelScheduleCopy` |  |  |  |
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
| POST | `/Poi/AddCustomPoiForWeb` |  |  |  |
| POST | `/Poi/Favorite/Post` |  |  |  |
| GET | `/Poi/GetPoiById` | guest | id | full POI, 163 KB (media, advertises, poiTickets) |
| GET | `/Poi/GetPoisByPoiNearBy` | guest | poiId | 12 full POIs, 248 KB |
| POST | `/Poi/MediaLike/Post` |  |  |  |
| GET | `/Poi/Search/Get` | guest | countryId, cityId, areaId, searchMode=location, categoryId (required), centerLongitude, centerLatitude, boundaryLongitude, boundaryLatitude, displayMode=poi | {state,totalCount,hasNextPage,result[],poiCount} |

## PoiClassification (4)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/PoiClassification/GetAll` | member | page | guest -> 002 Reject Guest Member |
| GET | `/PoiClassification/GetCustomPoiCategory` | member | - | guest -> 002 |
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
| POST | `/TravelSchedule/AddV2` |  |  |  |
| POST | `/TravelSchedule/Copy` |  |  |  |
| DELETE | `/TravelSchedule/DeleteDay` |  |  |  |
| GET | `/TravelSchedule/Get` |  |  |  |
| GET | `/TravelSchedule/GetCollaboration` |  |  |  |
| GET | `/TravelSchedule/GetCollaborationWithDetail` |  |  |  |
| GET | `/TravelSchedule/GetMyAndCollaboration` | token | updateTime, orderByColumn, sort | guest sees 2 demo schedules |
| GET | `/TravelSchedule/GetMyAndCollaborationWithTsdCount` | token | - | guest sees 2 demo schedules |
| GET | `/TravelSchedule/GetNote` |  |  |  |
| GET | `/TravelSchedule/GetSystemCoverList` | member | - | guest -> 002 |
| GET | `/TravelSchedule/GetUpdateTravelScheduleInfo` |  |  |  |
| GET | `/TravelSchedule/GetWithDetail` |  |  |  |
| PUT | `/TravelSchedule/UpdateNote` |  |  |  |
| PUT | `/TravelSchedule/UpdateV3` |  |  |  |

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
| POST | `/TravelScheduleDetail/Add` |  |  |  |
| POST | `/TravelScheduleDetail/AddByFavoritePoi` |  |  |  |
| POST | `/TravelScheduleDetail/Copy` |  |  |  |
| DELETE | `/TravelScheduleDetail/Delete` |  |  |  |
| GET | `/TravelScheduleDetail/Get` | token | travelScheduleId, TravelScheduleUpdateTime, isMyTravelSchedule | see design doc for preview flow |
| GET | `/TravelScheduleDetail/GetAddWhere` |  |  |  |
| GET | `/TravelScheduleDetail/GetAddWhereBestAll` |  |  |  |
| GET | `/TravelScheduleDetail/GetDetail` |  |  |  |
| GET | `/TravelScheduleDetail/GetEditInfo` |  |  |  |
| GET | `/TravelScheduleDetail/GetNote` |  |  |  |
| GET | `/TravelScheduleDetail/GetPreviewSettingByDay` |  |  |  |
| GET | `/TravelScheduleDetail/PreviewBestSortByDayV2` |  |  |  |
| PUT | `/TravelScheduleDetail/SaveBestSortByDayV2` |  |  |  |
| PUT | `/TravelScheduleDetail/SetCover` |  |  |  |
| PUT | `/TravelScheduleDetail/SetCustomRoute` |  |  |  |
| PUT | `/TravelScheduleDetail/SetDefaultRouteAndTsdAllDay` |  |  |  |
| PUT | `/TravelScheduleDetail/SetFlightRoute` |  |  |  |
| PUT | `/TravelScheduleDetail/SetRoute` |  |  |  |
| PUT | `/TravelScheduleDetail/Sort` |  |  |  |
| PUT | `/TravelScheduleDetail/Update` |  |  |  |
| PUT | `/TravelScheduleDetail/UpdateNote` |  |  |  |
| GET | `/TravelScheduleDetail/VerifyUpdateTime` | guest | TravelScheduleId, travelScheduleUpdateTime | 004 Update time conflict returns the current updateTime |

## TravelScheduleDetailRoute (3)

| Method | Path | Auth | Params (verified) | Notes |
|---|---|---|---|---|
| GET | `/TravelScheduleDetailRoute/GetFormulaTrafficTime` |  |  |  |
| GET | `/TravelScheduleDetailRoute/GetRouteDetail` |  |  |  |
| GET | `/TravelScheduleDetailRoute/GetRouteList` |  |  |  |

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
| GET | `/TravelScheduleUserLabel/Get` | member | - | guest -> 002 |
| PUT | `/TravelScheduleUserLabel/Sort` |  |  |  |
| PUT | `/TravelScheduleUserLabel/Update` |  |  |  |
