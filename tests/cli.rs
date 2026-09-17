//! End-to-end tests: the real binary against a local fixture server.
//!
//! Nothing here touches the network, the user's HOME, or a git config:
//! `CHICTRIP_AXI_BASE_URL` points at a `TcpListener` on localhost and
//! `CHICTRIP_AXI_AUTH_FILE` at a file under the temp directory.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

const MEMBER_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhYWFhYWFhYS0xMTExLTQxMTEtODExMS1hYWFhYWFhYWFhYWEiLCJleHAiOjE3OTczMTQyOTEsImlzcyI6IkNoaWNUcmlwQXBpIn0.c2lnbmF0dXJl";
const FRESH_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhYWFhYWFhYS0xMTExLTQxMTEtODExMS1hYWFhYWFhYWFhYWEiLCJleHAiOjE4MjczMTQyOTEsImlzcyI6IkNoaWNUcmlwQXBpIn0.c2lnbmF0dXJl";

const POI_SEARCH: &str = include_str!("fixtures/poi_search.json");
const POI_SEARCH_EMPTY: &str = include_str!("fixtures/poi_search_empty.json");
const POI_DETAIL: &str = include_str!("fixtures/poi_detail.json");
const TOUR_LIST: &str = include_str!("fixtures/tour_list.json");
const TRIP_LIST: &str = include_str!("fixtures/trip_list.json");
const TRIP_DETAIL: &str = include_str!("fixtures/trip_detail.json");
const TRIP_DETAIL_AFTER_ADD: &str = include_str!("fixtures/trip_detail_after_add.json");
const USER_LABELS: &str = include_str!("fixtures/user_labels.json");
const ADD_WHERE: &str = include_str!("fixtures/add_where.json");
const SYSTEM_COVERS: &str = include_str!("fixtures/system_covers.json");
const LOCATION_SEARCH: &str = include_str!("fixtures/location_search.json");

#[derive(Debug, Clone)]
struct Request {
    method: String,
    path: String,
    query: String,
    body: String,
}

impl Request {
    fn field(&self, name: &str) -> Option<String> {
        self.body.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key == name).then(|| value.replace('+', " "))
        })
    }
}

struct Server {
    base_url: String,
    log: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    /// The handler sees the request and how many times that path was already
    /// called, which is how the 003 and 004 tests change their answer.
    fn start<H>(handler: H) -> Server
    where
        H: Fn(&Request, usize) -> (u16, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
        let port = listener.local_addr().unwrap().port();
        let log: Arc<Mutex<Vec<Request>>> = Arc::new(Mutex::new(Vec::new()));
        let thread_log = Arc::clone(&log);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let request = match read_request(&mut stream) {
                    Some(request) => request,
                    None => continue,
                };
                let seen = {
                    let mut log = thread_log.lock().unwrap();
                    let seen = log.iter().filter(|r| r.path == request.path).count();
                    log.push(request.clone());
                    seen
                };
                let (status, body) = handler(&request, seen);
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        Server {
            base_url: format!("http://127.0.0.1:{port}/"),
            log,
        }
    }

    fn requests(&self) -> Vec<Request> {
        self.log.lock().unwrap().clone()
    }

    fn paths(&self) -> Vec<String> {
        self.requests().into_iter().map(|r| r.path).collect()
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut start = String::new();
    reader.read_line(&mut start).ok()?;
    let mut parts = start.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (target, String::new()),
    };
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body).ok()?;
    }
    Some(Request {
        method,
        path,
        query,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn ok(body: &str) -> (u16, String) {
    (200, body.to_string())
}

fn envelope(status: &str, data: &str, message: &str) -> (u16, String) {
    (
        200,
        format!(
            r#"{{"apiStatus":"{status}","data":{data},"message":{message},"requestId":"req-1"}}"#
        ),
    )
}

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "chictrip-axi-tests/{}-{name}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create sandbox");
        Sandbox { dir }
    }

    fn auth_file(&self) -> PathBuf {
        self.dir.join("auth.json")
    }

    fn write_auth(&self, access_token: &str, refresh_token: &str) {
        std::fs::write(
            self.auth_file(),
            format!(
                r#"{{"accessToken":"{access_token}","refreshToken":"{refresh_token}","memberId":"aaaaaaaa-1111-4111-8111-aaaaaaaaaaaa"}}"#
            ),
        )
        .expect("write auth file");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Derived from the index itself so adding a command does not churn tests.
fn command_index_header() -> String {
    format!(
        "commands[{}]{{command,summary}}:",
        chictrip_axi::commands::home::COMMAND_INDEX.len()
    )
}

fn run(server: &Server, auth_file: &Path, args: &[&str]) -> (String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_chictrip-axi"))
        .args(args)
        .env("CHICTRIP_AXI_BASE_URL", &server.base_url)
        .env("CHICTRIP_AXI_AUTH_FILE", auth_file)
        .env_remove("CHICTRIP_AXI_TOKEN")
        .output()
        .expect("run chictrip-axi");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

#[test]
fn poi_search_renders_a_toon_table_with_a_count_and_next_steps() {
    let server = Server::start(|_, _| ok(POI_SEARCH));
    let sandbox = Sandbox::new("poi-search");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["poi", "search", "asakusa", "--limit", "1"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        stdout,
        concat!(
            "count: 1 of 2\n",
            "pois[1]{id,name,category,rating,city,area}:\n",
            "  \"8a48a94c-495f-44da-be0d-e1d7564f2b07\",\u{6d45}\u{8349}\u{5bfa},enterTainment,4.5,Tokyo,Taito\n",
            "help[2]: \"Run `chictrip-axi poi view <id>` for hours and address and description\",\"Run `chictrip-axi trip add <trip-id> --day <n> --poi <id>` to put one in a trip\"\n"
        )
    );
    let request = &server.requests()[0];
    assert_eq!(request.path, "/PoiSearch/SearchByKeyword");
    assert!(request.query.contains("centerLongitude=0"));
    assert!(request.query.contains("centerLatitude=0"));
}

#[test]
fn poi_view_renders_a_detail_document_and_hides_the_sub_tables() {
    let server = Server::start(|_, _| ok(POI_DETAIL));
    let sandbox = Sandbox::new("poi-view");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["poi", "view", "8a48a94c-495f-44da-be0d-e1d7564f2b07"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.starts_with("poi:\n  id: \"8a48a94c-495f-44da-be0d-e1d7564f2b07\"\n"));
    assert!(stdout.contains("\n  rating: 4.5\n  reviews: 36177\n"));
    assert!(stdout.contains("\n  hours[2]: \"monday 00:00-24:00\",\"tuesday 00:00-24:00\"\n"));
    assert!(stdout.contains("\n  media: 1\n  tickets: 1\n"));
    assert!(stdout.contains("--full` for the full description"));
}

#[test]
fn the_json_flag_prints_the_same_document_as_one_line() {
    let server = Server::start(|_, _| ok(POI_SEARCH));
    let sandbox = Sandbox::new("json");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["poi", "search", "asakusa", "--json"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(stdout.lines().count(), 1);
    assert!(stdout.starts_with(r#"{"count":2,"pois":[{"id":"8a48a94c"#));
    assert!(
        stdout.contains(r#"{"id":"edd5509c-d852-41d1-9d27-0139d6d9f8f5","name":"Azumabashi pier""#)
    );
    assert!(stdout.trim_end().ends_with('}'));
}

#[test]
fn an_empty_search_is_definitive_and_still_succeeds() {
    let server = Server::start(|_, _| ok(POI_SEARCH_EMPTY));
    let sandbox = Sandbox::new("empty");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["poi", "search", "nothing-matches-this"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.starts_with("count: 0\npois[0]:\n"));
    assert!(stdout.contains("--near"));
}

#[test]
fn an_unknown_flag_is_a_usage_error_that_lists_the_valid_flags() {
    let server = Server::start(|_, _| ok(POI_SEARCH));
    let sandbox = Sandbox::new("usage");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["poi", "search", "asakusa", "--bogus"],
    );
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.starts_with("error: usage\n"));
    assert!(stdout.contains("--bogus"));
    assert!(stdout.contains("flags[7]:"));
    assert!(stdout.contains("--near"));
    assert!(stdout.contains("--limit"));
    assert!(server.requests().is_empty(), "usage errors never call out");
}

#[test]
fn the_guest_token_refuses_member_commands_before_any_request() {
    let server = Server::start(|_, _| ok(TRIP_LIST));
    let sandbox = Sandbox::new("guest");
    let (stdout, code) = run(&server, &sandbox.auth_file(), &["trip", "list"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: auth_required\n"));
    assert!(stdout.contains("auth set --from-json -"));
    assert!(server.requests().is_empty(), "no request without a member");
}

#[test]
fn an_expired_token_is_refreshed_and_the_request_replayed() {
    let server = Server::start(|request, seen| match request.path.as_str() {
        "/TravelSchedule/GetMyAndCollaboration" if seen == 0 => envelope("003", "null", "null"),
        "/TravelSchedule/GetMyAndCollaboration" => ok(TRIP_LIST),
        "/Token/Refresh" => envelope(
            "001",
            &format!(
                r#"{{"accessToken":"{FRESH_TOKEN}","refreshToken":"refresh-2","memberId":"aaaaaaaa-1111-4111-8111-aaaaaaaaaaaa"}}"#
            ),
            "null",
        ),
        _ => envelope("002", "null", r#""404""#),
    });
    let sandbox = Sandbox::new("refresh");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(&server, &sandbox.auth_file(), &["trip", "list"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.starts_with("count: 2\n"));
    assert_eq!(
        server.paths(),
        vec![
            "/TravelSchedule/GetMyAndCollaboration",
            "/Token/Refresh",
            "/TravelSchedule/GetMyAndCollaboration",
        ]
    );

    let refresh = &server.requests()[1];
    assert_eq!(refresh.method, "POST");
    assert_eq!(refresh.field("refreshToken").as_deref(), Some("refresh-1"));

    let stored = std::fs::read_to_string(sandbox.auth_file()).unwrap();
    assert!(stored.contains(FRESH_TOKEN));
    assert!(stored.contains("refresh-2"));
    assert!(server.requests()[2].body.is_empty());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(sandbox.auth_file())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

#[test]
fn an_update_time_conflict_is_re_read_and_retried_once() {
    let server = Server::start(|request, seen| match request.path.as_str() {
        "/TravelScheduleDetail/VerifyUpdateTime" if seen == 0 => {
            envelope("001", r#"{"updateTime":100}"#, "null")
        }
        "/TravelScheduleDetail/VerifyUpdateTime" => {
            envelope("004", r#"{"updateTime":150}"#, r#""Update time conflict""#)
        }
        "/TravelScheduleDetail/Get" if seen == 0 => ok(TRIP_DETAIL),
        "/TravelScheduleDetail/Get" => ok(TRIP_DETAIL_AFTER_ADD),
        "/Poi/GetPoiById" => ok(POI_DETAIL),
        "/TravelScheduleDetail/GetAddWhere" => ok(ADD_WHERE),
        "/TravelScheduleDetail/Add" if seen == 0 => {
            envelope("004", r#"{"updateTime":150}"#, r#""Update time conflict""#)
        }
        "/TravelScheduleDetail/Add" => {
            envelope("001", r#"{"travelScheduleUpdateTime":1789710600}"#, "null")
        }
        _ => envelope("002", "null", r#""404""#),
    });
    let sandbox = Sandbox::new("conflict");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "add",
            "3c1d0a2e-1111-4111-8111-111111111111",
            "--day",
            "1",
            "--poi",
            "8a48a94c-495f-44da-be0d-e1d7564f2b07",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("added[1]{poi_id,name,tsd_id,seq}:"));
    assert!(stdout.contains("\"c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb\",2"));
    assert!(stdout.contains("update_time: 1789710600"));

    let adds: Vec<Request> = server
        .requests()
        .into_iter()
        .filter(|r| r.path == "/TravelScheduleDetail/Add")
        .collect();
    assert_eq!(adds.len(), 2, "the conflict is retried exactly once");
    assert_eq!(
        adds[0].field("TravelScheduleUpdateTime").as_deref(),
        Some("100")
    );
    assert_eq!(
        adds[1].field("TravelScheduleUpdateTime").as_deref(),
        Some("150")
    );
    assert_eq!(adds[1].field("AddWhereId").as_deref(), Some("slot-1-last"));
}

#[test]
fn trip_add_skips_a_poi_already_in_the_day_without_writing() {
    let server = Server::start(|request, _| match request.path.as_str() {
        "/TravelScheduleDetail/VerifyUpdateTime" => {
            envelope("001", r#"{"updateTime":100}"#, "null")
        }
        "/TravelScheduleDetail/Get" => ok(TRIP_DETAIL),
        _ => envelope("002", "null", r#""404""#),
    });
    let sandbox = Sandbox::new("dup");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "add",
            "3c1d0a2e-1111-4111-8111-111111111111",
            "--day",
            "1",
            "--poi",
            "edd5509c-d852-41d1-9d27-0139d6d9f8f5",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("added[0]:"));
    assert!(stdout.contains("already in day 1"));
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/Add".to_string()),
        "a skipped POI is never written"
    );
}

#[test]
fn trip_create_is_a_no_op_when_the_same_trip_already_exists() {
    let server = Server::start(|request, _| match request.path.as_str() {
        "/TravelSchedule/GetMyAndCollaboration" => ok(TRIP_LIST),
        "/TravelScheduleUserLabel/Get" => ok(USER_LABELS),
        _ => envelope("002", "null", r#""404""#),
    });
    let sandbox = Sandbox::new("create-noop");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "create",
            "--name",
            "Tokyo temples",
            "--start",
            "2026-10-01",
            "--end",
            "2026-10-03",
            "--location",
            "7,7,0",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("note: \"already exists (no-op)\""));
    assert!(stdout.contains("id: \"3c1d0a2e-1111-4111-8111-111111111111\""));
    assert!(
        !server
            .paths()
            .contains(&"/TravelSchedule/AddV2".to_string()),
        "the no-op never writes"
    );
}

#[test]
fn trip_create_sends_the_form_the_api_documents() {
    let server = Server::start(|request, _| match request.path.as_str() {
        "/TravelSchedule/GetMyAndCollaboration" => ok(TRIP_LIST),
        "/TravelScheduleUserLabel/Get" => ok(USER_LABELS),
        "/TravelSchedule/GetSystemCoverList" => ok(SYSTEM_COVERS),
        "/TravelSchedule/AddV2" => envelope(
            "001",
            r#"{"id":"9f0e1d2c-5555-4555-8555-555555555555","name":"Osaka","updateTime":1789720000,"permission":"Owner"}"#,
            "null",
        ),
        _ => envelope("002", "null", r#""404""#),
    });
    let sandbox = Sandbox::new("create");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "create",
            "--name",
            "Osaka",
            "--start",
            "2026-10-01",
            "--end",
            "2026-10-03",
            "--location",
            "7,7,0",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("days: 3"));

    let add = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelSchedule/AddV2")
        .expect("AddV2 was called");
    assert_eq!(
        add.field("CoverMediaId").as_deref(),
        Some("c0ffee00-1111-4111-8111-111111111111")
    );
    assert!(!add.body.contains("destinationList"));
    assert_eq!(add.field("StartDate").as_deref(), Some("2026%2F10%2F01"));
    assert_eq!(add.field("TotalDay").as_deref(), Some("3"));
    assert_eq!(add.field("ViewMode").as_deref(), Some("DetailMode"));
    assert_eq!(
        add.field("TravelScheduleUserLabelId").as_deref(),
        Some("1eb4c5b1-4444-4444-8444-444444444444")
    );
    assert!(add.body.contains("LocationKey%5B%5D=7%2C7%2C0"));
}

#[test]
fn trip_create_without_a_location_is_a_usage_error_before_any_request() {
    let server = Server::start(|_, _| ok(TRIP_LIST));
    let sandbox = Sandbox::new("create-no-location");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "create",
            "--name",
            "Osaka",
            "--start",
            "2026-10-01",
            "--end",
            "2026-10-03",
        ],
    );
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.starts_with("error: usage\n"));
    assert!(
        stdout.contains("were not provided: --location <KEY>"),
        "the message names the missing flag: {stdout}"
    );
    assert!(server.requests().is_empty(), "usage errors never call out");
}

#[test]
fn location_search_lists_the_destination_keys_trip_create_needs() {
    let server = Server::start(|_, _| ok(LOCATION_SEARCH));
    let sandbox = Sandbox::new("location");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["location", "search", "Tokyo"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.starts_with(concat!(
            "count: 2\n",
            "locations[2]{name,full_name,key}:\n",
            "  Tokyo,Japan/Tokyo,\"7,7,0\"\n",
            "  Shinjuku,Japan/Tokyo/Shinjuku,\"7,7,6\"\n",
        )),
        "{stdout}"
    );
    assert!(stdout.contains("--location <key>"));
    let request = &server.requests()[0];
    assert_eq!(request.path, "/ExpertTour/SearchLocation");
    assert!(request.query.contains("keyword=Tokyo"));
}

#[test]
fn trip_preview_reads_a_shared_trip_as_a_guest() {
    let server = Server::start(|_, _| ok(TRIP_DETAIL));
    let sandbox = Sandbox::new("preview");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "preview", "3c1d0a2e-1111-4111-8111-111111111111"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        stdout,
        concat!(
            "trip:\n",
            "  id: \"3c1d0a2e-1111-4111-8111-111111111111\"\n",
            "  name: Tokyo temples\n",
            "  start: 2026/10/01\n",
            "  end: 2026/10/03\n",
            "  days: 3\n",
            "stops[1]{day,seq,arrive,stay_min,name,type,city,poi_id}:\n",
            "  1,1,\"09:00\",60,Azumabashi pier,basic,Tokyo,\"edd5509c-d852-41d1-9d27-0139d6d9f8f5\"\n",
            "help[2]: \"Run `chictrip-axi poi view <poi_id>` for a stop\",\"Run `chictrip-axi trip add <my-trip-id> --day <n> --poi <poi_id>` to copy a stop into my own trip\"\n"
        )
    );
    assert_eq!(server.requests().len(), 1, "preview is a single call");
    let request = &server.requests()[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/TravelScheduleDetail/Preview");
    assert!(
        request
            .query
            .contains("TravelScheduleId=3c1d0a2e-1111-4111-8111-111111111111"),
        "{}",
        request.query
    );
}

#[test]
fn trip_preview_of_a_deleted_trip_is_not_found() {
    let server =
        Server::start(|_, _| envelope("011", "null", "\"TravelSchedule has been deleted\""));
    let sandbox = Sandbox::new("preview-deleted");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "preview", "00000000-0000-4000-8000-000000000000"],
    );
    assert_eq!(code, 1, "{stdout}");
    assert!(
        stdout.starts_with("error: not_found\nmessage: TravelSchedule has been deleted\n"),
        "{stdout}"
    );
}

#[test]
fn trip_preview_rejects_a_day_outside_the_trip() {
    let server = Server::start(|_, _| ok(TRIP_DETAIL));
    let sandbox = Sandbox::new("preview-day");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "preview",
            "3c1d0a2e-1111-4111-8111-111111111111",
            "--day",
            "4",
        ],
    );
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: not_found\n"), "{stdout}");
    assert!(
        stdout.contains("day 4 is outside this itinerary"),
        "{stdout}"
    );
    assert_eq!(server.requests().len(), 1, "preview is a single call");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "preview",
            "3c1d0a2e-1111-4111-8111-111111111111",
            "--day",
            "2",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("\nstops[0]:\n"), "{stdout}");
}

#[test]
fn an_unknown_path_is_reported_as_not_found() {
    let server = Server::start(|_, _| (404, "Not Found".to_string()));
    let sandbox = Sandbox::new("404");
    let (stdout, code) = run(&server, &sandbox.auth_file(), &["tour", "list"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: not_found\n"));
}

#[test]
fn the_home_view_shows_live_tours_and_the_command_index_for_a_guest() {
    let server = Server::start(|_, _| ok(TOUR_LIST));
    let sandbox = Sandbox::new("home");
    let (stdout, code) = run(&server, &sandbox.auth_file(), &[]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("auth: guest\n"));
    assert!(stdout.contains("tours[2]{id,name,destination,expert,likes}:"));
    assert!(stdout.contains(&command_index_header()));
}

#[test]
fn a_failing_home_view_still_prints_the_command_index_and_exits_1() {
    let server =
        Server::start(|_, _| envelope("014", "null", r#""chicTrip is under maintenance""#));
    let sandbox = Sandbox::new("home-down");
    let (stdout, code) = run(&server, &sandbox.auth_file(), &[]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("error: api_error\n"));
    assert!(stdout.contains("under maintenance"));
    assert!(stdout.contains(&command_index_header()));
}

#[test]
fn an_unknown_fields_name_is_a_usage_error_listing_the_valid_names() {
    let server = Server::start(|_, _| ok(POI_SEARCH));
    let sandbox = Sandbox::new("fields");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["poi", "search", "asakusa", "--fields", "id,nope"],
    );
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.contains("valid names: id,name,category,rating,city,area"));
}

#[test]
fn auth_set_stores_the_browser_json_and_auth_clear_is_a_no_op_when_empty() {
    let server = Server::start(|_, _| ok(POI_SEARCH));
    let sandbox = Sandbox::new("auth");
    let auth_file = sandbox.auth_file();

    let output = Command::new(env!("CARGO_BIN_EXE_chictrip-axi"))
        .args(["auth", "set", "--from-json", "-"])
        .env("CHICTRIP_AXI_BASE_URL", &server.base_url)
        .env("CHICTRIP_AXI_AUTH_FILE", &auth_file)
        .env_remove("CHICTRIP_AXI_TOKEN")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            let payload = format!(
                r#"{{"accessToken":"{MEMBER_TOKEN}","refreshToken":"r","memberId":"m-1"}}"#
            );
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(payload.as_bytes())?;
            child.wait_with_output()
        })
        .expect("auth set");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("member_id: \"m-1\""));
    assert!(stdout.contains("access_token_expires: \"2026-12-15T05:58:11Z\""));
    assert!(auth_file.exists());

    let (stdout, code) = run(&server, &auth_file, &["auth", "clear"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("cleared"));
    assert!(!auth_file.exists());

    let (stdout, code) = run(&server, &auth_file, &["auth", "clear"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("nothing stored (no-op)"));
}
