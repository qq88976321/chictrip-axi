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
const TRIP_DETAIL_FULL: &str = include_str!("fixtures/trip_detail_full.json");
const USER_LABELS: &str = include_str!("fixtures/user_labels.json");
const ADD_WHERE: &str = include_str!("fixtures/add_where.json");
const ADD_WHERE_FULL: &str = include_str!("fixtures/add_where_full.json");
const ADD_WHERE_PREPENDED: &str = include_str!("fixtures/add_where_prepended.json");
const EDIT_INFO: &str = include_str!("fixtures/edit_info.json");
const EDIT_INFO_AFTER: &str = include_str!("fixtures/edit_info_after.json");
const ROUTE_LIST_TRANSIT: &str = include_str!("fixtures/route_list_transit.json");
const ROUTE_LIST_DRIVING: &str = include_str!("fixtures/route_list_driving.json");
const ROUTE_LIST_CUSTOM: &str = include_str!("fixtures/route_list_custom.json");
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

    /// A working directory for the commands that write relative paths.
    fn project(&self) -> PathBuf {
        let dir = self.dir.join("project");
        std::fs::create_dir_all(&dir).expect("create project dir");
        dir
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

/// A run with a working directory of its own, for the commands that write
/// files relative to it. Port 9 is the discard service and nothing listens
/// there, so a command that unexpectedly calls out fails instead of
/// reaching the real API.
fn run_in(sandbox: &Sandbox, cwd: &Path, env: &[(&str, &str)], args: &[&str]) -> (String, i32) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chictrip-axi"));
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", &sandbox.dir)
        .env("CHICTRIP_AXI_AUTH_FILE", sandbox.auth_file())
        .env("CHICTRIP_AXI_BASE_URL", "http://127.0.0.1:9/")
        .env_remove("CHICTRIP_AXI_TOKEN")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("CLAUDE_CONFIG_DIR");
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().expect("run chictrip-axi");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        output.status.code().unwrap_or(-1),
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
        "/TravelScheduleDetail/Add" => envelope(
            "001",
            r#"{"tsdInfo":{"id":"c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb"},"travelScheduleUpdateTime":1789710600}"#,
            "null",
        ),
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

/// The fixture server for the milestone 4 trip commands: a current update
/// time and the three-day probe trip.
fn trip_server<H>(handler: H) -> Server
where
    H: Fn(&Request, usize) -> Option<(u16, String)> + Send + 'static,
{
    Server::start(move |request, seen| {
        if let Some(answer) = handler(request, seen) {
            return answer;
        }
        match request.path.as_str() {
            "/TravelScheduleDetail/VerifyUpdateTime" => {
                envelope("001", r#"{"updateTime":100}"#, "null")
            }
            "/TravelScheduleDetail/Get" => ok(TRIP_DETAIL_FULL),
            _ => envelope("002", "null", r#""404""#),
        }
    })
}

#[test]
fn trip_view_full_shows_pinned_times_notes_and_the_trip_note() {
    let server = trip_server(|_, _| None);
    let sandbox = Sandbox::new("view-full");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "view",
            "3c1d0a2e-1111-4111-8111-111111111111",
            "--full",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("\n  note: Buy the 72h subway pass at Narita\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "stops[4]{day,seq,arrive,stay_min,name,type,tsd_id,city,poi_id,note,traffic,traffic_min,depart,category,flight}:"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("\n  1,2,\"10:30\",90,\"Senso-ji\","),
        "the pinned arrival wins over the computed one: {stdout}"
    );
    assert!(
        stdout.contains("\"Reservation 19:00\",Transit,12,\"12:00\",enterTainment,null\n"),
        "{stdout}"
    );
    assert!(stdout.contains("--full` for notes and legs"), "{stdout}");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "view", "3c1d0a2e-1111-4111-8111-111111111111"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("stops[4]{day,seq,arrive,stay_min,name,type,tsd_id,city,poi_id}:"),
        "the default schema is unchanged: {stdout}"
    );
}

#[test]
fn trip_preview_full_keeps_the_detail_columns_without_the_tsd_id() {
    let server = Server::start(|_, _| ok(TRIP_DETAIL_FULL));
    let sandbox = Sandbox::new("preview-full");
    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "preview",
            "3c1d0a2e-1111-4111-8111-111111111111",
            "--full",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains(
            "stops[4]{day,seq,arrive,stay_min,name,type,city,poi_id,note,traffic,traffic_min,depart,category,flight}:"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("tsd_id"), "{stdout}");
    assert!(
        !stdout.contains("72h subway pass"),
        "Preview hides the trip note: {stdout}"
    );
}

#[test]
fn trip_add_inserts_before_the_right_slot_and_chains_the_batch() {
    let server = trip_server(|request, seen| match request.path.as_str() {
        "/Poi/GetPoiById" => Some(ok(POI_DETAIL)),
        "/TravelScheduleDetail/GetAddWhere" => Some(ok(ADD_WHERE_FULL)),
        "/TravelScheduleDetail/Add" if seen == 0 => Some(envelope(
            "001",
            r#"{"tsdInfo":{"id":"new-1"},"travelScheduleUpdateTime":101}"#,
            "null",
        )),
        "/TravelScheduleDetail/Add" => Some(envelope(
            "001",
            r#"{"tsdInfo":{"id":"new-2"},"travelScheduleUpdateTime":102}"#,
            "null",
        )),
        _ => None,
    });
    let sandbox = Sandbox::new("add-after");
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
            "p-1",
            "--poi",
            "p-2",
            "--after",
            "b2d11753-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    let adds: Vec<Request> = server
        .requests()
        .into_iter()
        .filter(|r| r.path == "/TravelScheduleDetail/Add")
        .collect();
    assert_eq!(adds.len(), 2, "{stdout}");
    for add in &adds {
        assert_eq!(
            add.field("AddWhereId").as_deref(),
            Some("c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb"),
            "the anchor slot does not move while the batch runs"
        );
    }
    assert!(
        stdout.contains("added[2]{poi_id,name,tsd_id,seq}:"),
        "{stdout}"
    );
    assert!(stdout.contains("\"p-1\","), "{stdout}");
    assert!(stdout.contains("new-1"), "{stdout}");
    assert!(stdout.contains("new-2"), "{stdout}");
}

#[test]
fn trip_add_position_first_prepends_then_chains_behind_itself() {
    let server = trip_server(|request, seen| match request.path.as_str() {
        "/Poi/GetPoiById" => Some(ok(POI_DETAIL)),
        "/TravelScheduleDetail/GetAddWhere" if seen == 0 => Some(ok(ADD_WHERE_FULL)),
        "/TravelScheduleDetail/GetAddWhere" => Some(ok(ADD_WHERE_PREPENDED)),
        "/TravelScheduleDetail/Add" => Some(envelope(
            "001",
            r#"{"tsdInfo":{"id":"new-1"},"travelScheduleUpdateTime":101}"#,
            "null",
        )),
        _ => None,
    });
    let sandbox = Sandbox::new("add-first");
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
            "p-3",
            "--poi",
            "p-4",
            "--position",
            "first",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    let slots: Vec<Option<String>> = server
        .requests()
        .into_iter()
        .filter(|r| r.path == "/TravelScheduleDetail/Add")
        .map(|r| r.field("AddWhereId"))
        .collect();
    assert_eq!(
        slots,
        vec![
            Some("start".to_string()),
            Some("b2d11753-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string())
        ],
        "first prepends, then chains behind what it just inserted"
    );
}

#[test]
fn trip_add_after_a_stop_of_another_day_is_not_found_before_any_write() {
    let server = trip_server(|_, _| None);
    let sandbox = Sandbox::new("add-after-miss");
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
            "p-1",
            "--after",
            "e0000000-dddd-4ddd-8ddd-dddddddddddd",
        ],
    );
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: not_found\n"), "{stdout}");
    assert!(stdout.contains("is not in day 1 of this trip"), "{stdout}");
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/Add".to_string()),
        "a bad anchor never writes"
    );
}

#[test]
fn trip_add_rejects_after_together_with_position() {
    let server = trip_server(|_, _| None);
    let sandbox = Sandbox::new("add-after-conflict");
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
            "p-1",
            "--after",
            "b2d11753-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            "--position",
            "first",
        ],
    );
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.starts_with("error: usage\n"), "{stdout}");
    assert!(server.requests().is_empty(), "usage errors never call out");
}

const STOP: &str = "c9e2f004-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const TRIP: &str = "3c1d0a2e-1111-4111-8111-111111111111";

#[test]
fn trip_edit_sends_the_merged_update_form_and_re_reads() {
    let server = trip_server(|request, seen| match request.path.as_str() {
        "/TravelScheduleDetail/GetEditInfo" if seen == 0 => Some(ok(EDIT_INFO)),
        "/TravelScheduleDetail/GetEditInfo" => Some(ok(EDIT_INFO_AFTER)),
        "/TravelScheduleDetail/Update" => Some(envelope("001", "1789710700", "null")),
        _ => None,
    });
    let sandbox = Sandbox::new("edit");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "edit",
            TRIP,
            "--stop",
            STOP,
            "--stay",
            "120",
            "--category",
            "food",
        ],
    );
    assert_eq!(code, 0, "{stdout}");

    let update = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelScheduleDetail/Update")
        .expect("Update was called");
    assert_eq!(update.method, "PUT");
    assert_eq!(update.field("StayTime").as_deref(), Some("120"));
    assert_eq!(update.field("IsUseCustomArrivalTime").as_deref(), Some("1"));
    assert_eq!(
        update.field("CustomArrivalTime").as_deref(),
        Some("10%3A30")
    );
    assert_eq!(
        update.field("IsUseCustomDepartureTime").as_deref(),
        Some("1")
    );
    assert_eq!(
        update.field("PoiClassificationId").as_deref(),
        Some("9449daa2-12ae-4a65-aa31-8ef93283763f")
    );
    assert_eq!(
        update.field("travelScheduleUpdateTime").as_deref(),
        Some("100")
    );

    assert!(stdout.contains("\n  category: food\n"), "{stdout}");
    assert!(stdout.contains("\n  type: basic\n"), "{stdout}");
    assert!(
        stdout.contains("\n  arrive: \"10:30\"\n  arrive_custom: true\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\n  stay_min: 120\n"), "{stdout}");
    assert!(
        stdout.contains("\nchanged[2]: category,stay_min\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\nupdate_time: 1789710700\n"), "{stdout}");
    assert_eq!(
        server.paths(),
        vec![
            "/TravelScheduleDetail/VerifyUpdateTime",
            "/TravelScheduleDetail/GetEditInfo",
            "/TravelScheduleDetail/Update",
            "/TravelScheduleDetail/GetEditInfo",
        ]
    );
}

#[test]
fn trip_edit_with_the_current_values_is_a_no_op() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelScheduleDetail/GetEditInfo" => Some(ok(EDIT_INFO)),
        _ => None,
    });
    let sandbox = Sandbox::new("edit-noop");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip", "edit", TRIP, "--stop", STOP, "--stay", "90", "--arrive", "10:30",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("\nchanged[0]:\n"), "{stdout}");
    assert!(
        stdout.contains("note: \"already as requested (no-op)\""),
        "{stdout}"
    );
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/Update".to_string()),
        "a no-op never writes"
    );
}

#[test]
fn trip_edit_rejects_bad_flags_before_any_request() {
    let server = trip_server(|_, _| None);
    let sandbox = Sandbox::new("edit-usage");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    for args in [
        vec!["trip", "edit", TRIP, "--stop", STOP, "--arrive", "25:00"],
        vec!["trip", "edit", TRIP, "--stop", STOP, "--stay", "2000"],
        vec!["trip", "edit", TRIP, "--stop", STOP],
    ] {
        let (stdout, code) = run(&server, &sandbox.auth_file(), &args);
        assert_eq!(code, 2, "{args:?} {stdout}");
        assert!(stdout.starts_with("error: usage\n"), "{stdout}");
    }
    assert!(server.requests().is_empty(), "usage errors never call out");
}

#[test]
fn trip_edit_with_an_unknown_category_lists_the_live_vocabulary() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelScheduleDetail/GetEditInfo" => Some(ok(EDIT_INFO)),
        _ => None,
    });
    let sandbox = Sandbox::new("edit-category");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "edit", TRIP, "--stop", STOP, "--category", "hotel"],
    );
    assert_eq!(code, 2, "{stdout}");
    assert!(
        stdout.contains("valid categories: enterTainment,food,"),
        "{stdout}"
    );
    assert!(stdout.contains("landing"), "{stdout}");
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/Update".to_string()),
        "an unknown category never writes"
    );
}

#[test]
fn trip_edit_on_a_stop_of_another_trip_is_not_found() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelScheduleDetail/GetEditInfo" => {
            Some(envelope("002", "null", r#""TSD Id not found""#))
        }
        _ => None,
    });
    let sandbox = Sandbox::new("edit-missing");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "edit",
            TRIP,
            "--stop",
            "00000000-0000-0000-0000-000000000000",
            "--stay",
            "30",
        ],
    );
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: not_found\n"), "{stdout}");
    assert!(stdout.contains("is not in trip"), "{stdout}");
    assert!(stdout.contains("trip view"), "{stdout}");
}

#[test]
fn trip_edit_retries_once_on_an_update_time_conflict() {
    let server = trip_server(|request, seen| match request.path.as_str() {
        "/TravelScheduleDetail/VerifyUpdateTime" if seen == 0 => {
            Some(envelope("001", r#"{"updateTime":100}"#, "null"))
        }
        "/TravelScheduleDetail/VerifyUpdateTime" => Some(envelope(
            "004",
            r#"{"updateTime":150}"#,
            r#""Update time conflict""#,
        )),
        "/TravelScheduleDetail/GetEditInfo" if seen == 0 => Some(ok(EDIT_INFO)),
        "/TravelScheduleDetail/GetEditInfo" => Some(ok(EDIT_INFO_AFTER)),
        "/TravelScheduleDetail/Update" if seen == 0 => Some(envelope(
            "004",
            r#"{"updateTime":150}"#,
            r#""Update time conflict""#,
        )),
        "/TravelScheduleDetail/Update" => Some(envelope("001", "1789710700", "null")),
        _ => None,
    });
    let sandbox = Sandbox::new("edit-conflict");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "edit", TRIP, "--stop", STOP, "--stay", "120"],
    );
    assert_eq!(code, 0, "{stdout}");
    let updates: Vec<Request> = server
        .requests()
        .into_iter()
        .filter(|r| r.path == "/TravelScheduleDetail/Update")
        .collect();
    assert_eq!(updates.len(), 2, "the conflict is retried exactly once");
    assert_eq!(
        updates[0].field("travelScheduleUpdateTime").as_deref(),
        Some("100")
    );
    assert_eq!(
        updates[1].field("travelScheduleUpdateTime").as_deref(),
        Some("150")
    );
}

#[test]
fn trip_note_reads_sets_and_clears_the_stop_note() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelScheduleDetail/UpdateNote" => Some(envelope("001", "1789710800", "null")),
        _ => None,
    });
    let sandbox = Sandbox::new("note-stop");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "note", TRIP, "--stop", STOP],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("\nname: \"Senso-ji\"\n"), "{stdout}");
    assert!(
        stdout.contains("\nnote: \"Reservation 19:00\"\n"),
        "{stdout}"
    );
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/UpdateNote".to_string()),
        "a read never writes"
    );

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "note",
            TRIP,
            "--stop",
            STOP,
            "--set",
            "Reservation 19:00",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: \"unchanged (no-op)\""), "{stdout}");
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/UpdateNote".to_string()),
        "the same text again never writes"
    );

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "note", TRIP, "--stop", STOP, "--set", "new text"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: set\n"), "{stdout}");
    assert!(stdout.contains("update_time: 1789710800\n"), "{stdout}");
    let write = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelScheduleDetail/UpdateNote")
        .expect("UpdateNote was called");
    assert_eq!(write.method, "PUT");
    assert_eq!(write.field("TsdId").as_deref(), Some(STOP));
    assert_eq!(write.field("Note").as_deref(), Some("new text"));
    assert_eq!(
        write.field("TravelScheduleUpdateTime").as_deref(),
        Some("100")
    );

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "note", TRIP, "--stop", STOP, "--clear"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("\nnote: \"\"\n"), "{stdout}");
    assert!(stdout.contains("status: cleared\n"), "{stdout}");
    let cleared = server
        .requests()
        .into_iter()
        .rfind(|r| r.path == "/TravelScheduleDetail/UpdateNote")
        .expect("UpdateNote was called");
    assert_eq!(cleared.field("Note").as_deref(), Some(""));
}

#[test]
fn trip_note_on_the_trip_uses_the_trip_endpoint() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelSchedule/UpdateNote" => Some(envelope("001", "1789710900", "null")),
        _ => None,
    });
    let sandbox = Sandbox::new("note-trip");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(&server, &sandbox.auth_file(), &["trip", "note", TRIP]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("\nnote: Buy the 72h subway pass at Narita\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("stop_id"), "{stdout}");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "note", TRIP, "--set", "Pick the pass up at Narita"],
    );
    assert_eq!(code, 0, "{stdout}");
    let write = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelSchedule/UpdateNote")
        .expect("UpdateNote was called");
    assert_eq!(write.method, "PUT");
    assert_eq!(write.field("id").as_deref(), Some(TRIP));
    assert_eq!(
        write.field("note").as_deref(),
        Some("Pick the pass up at Narita")
    );
    assert_eq!(write.field("updateTime").as_deref(), Some("100"));
}

#[test]
fn trip_note_rejects_set_together_with_clear_before_any_request() {
    let server = trip_server(|_, _| None);
    let sandbox = Sandbox::new("note-usage");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "note", TRIP, "--set", "x", "--clear"],
    );
    assert_eq!(code, 2, "{stdout}");
    assert!(stdout.starts_with("error: usage\n"), "{stdout}");
    assert!(server.requests().is_empty(), "usage errors never call out");
}

/// The third stop of day 1 in the fixture: a Custom 25 minute leg.
const CUSTOM_STOP: &str = "d0000000-cccc-4ccc-8ccc-cccccccccccc";

#[test]
fn trip_leg_lists_transit_routes_with_fares_by_default() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelScheduleDetailRoute/GetRouteList" => Some(ok(ROUTE_LIST_TRANSIT)),
        _ => None,
    });
    let sandbox = Sandbox::new("leg-list");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "leg", TRIP, "--stop", STOP],
    );
    assert_eq!(code, 0, "{stdout}");
    let list = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelScheduleDetailRoute/GetRouteList")
        .expect("GetRouteList was called");
    assert!(list.query.contains("trafficType=Transit"), "{}", list.query);
    assert!(
        list.query.contains("tsdRouteDetailId=route-2"),
        "{}",
        list.query
    );
    assert!(stdout.contains("\n  from: Azumabashi pier\n"), "{stdout}");
    assert!(stdout.contains("\nmode: Transit\ncount: 2\n"), "{stdout}");
    assert!(
        stdout.contains("routes[2]{route_id,minutes,km,summary,fare,selected}:"),
        "{stdout}"
    );
    assert!(
        stdout.contains("\n  \"t-1\",14,3.2,null,JPY 180,true\n"),
        "{stdout}"
    );
    assert!(stdout.contains("--route <route_id>"), "{stdout}");
}

#[test]
fn trip_leg_on_a_days_first_stop_is_not_found() {
    let server = trip_server(|_, _| None);
    let sandbox = Sandbox::new("leg-first");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "leg",
            TRIP,
            "--stop",
            "b2d11753-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        ],
    );
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: not_found\n"), "{stdout}");
    assert!(stdout.contains("first stop of day 1"), "{stdout}");
    assert!(
        stdout.contains(STOP),
        "the help names the next stop: {stdout}"
    );
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetailRoute/GetRouteList".to_string()),
        "a stop without a leg never asks for routes"
    );
}

#[test]
fn trip_leg_picks_a_route_and_skips_one_already_selected() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelScheduleDetailRoute/GetRouteList" => Some(ok(ROUTE_LIST_DRIVING)),
        "/TravelScheduleDetail/SetRoute" => Some(envelope("001", "1789710950", "null")),
        _ => None,
    });
    let sandbox = Sandbox::new("leg-route");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "leg", TRIP, "--stop", STOP, "--route", "r-1"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("note: \"already selected (no-op)\""),
        "{stdout}"
    );
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/SetRoute".to_string()),
        "the selected route is never written again"
    );

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "leg", TRIP, "--stop", STOP, "--route", "r-2"],
    );
    assert_eq!(code, 0, "{stdout}");
    let write = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelScheduleDetail/SetRoute")
        .expect("SetRoute was called");
    assert_eq!(write.method, "PUT");
    assert_eq!(write.field("TsdRouteDetailId").as_deref(), Some("route-2"));
    assert_eq!(write.field("PoiRouteDetailId").as_deref(), Some("r-2"));
    assert!(stdout.contains("update_time: 1789710950"), "{stdout}");
    assert_eq!(
        server.paths().last().map(String::as_str),
        Some("/TravelScheduleDetail/Get"),
        "a write is read back"
    );
}

#[test]
fn trip_leg_sets_a_custom_or_flight_duration_unless_already_set() {
    let server = trip_server(|request, _| match request.path.as_str() {
        "/TravelScheduleDetailRoute/GetRouteList" => Some(ok(ROUTE_LIST_CUSTOM)),
        "/TravelScheduleDetail/SetCustomRoute" => Some(envelope("001", "1789711000", "null")),
        "/TravelScheduleDetail/SetFlightRoute" => Some(envelope("001", "1789711001", "null")),
        _ => None,
    });
    let sandbox = Sandbox::new("leg-custom");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "leg",
            TRIP,
            "--stop",
            CUSTOM_STOP,
            "--custom",
            "25",
            "--note",
            "hotel shuttle",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("note: \"already set (no-op)\""), "{stdout}");
    assert!(
        !server
            .paths()
            .contains(&"/TravelScheduleDetail/SetCustomRoute".to_string()),
        "the same leg is never written again"
    );

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &["trip", "leg", TRIP, "--stop", CUSTOM_STOP, "--custom", "30"],
    );
    assert_eq!(code, 0, "{stdout}");
    let write = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelScheduleDetail/SetCustomRoute")
        .expect("SetCustomRoute was called");
    assert_eq!(write.method, "PUT");
    assert_eq!(write.field("Duration").as_deref(), Some("30"));
    assert_eq!(write.field("Note").as_deref(), Some(""));
    assert_eq!(write.field("TsdRouteDetailId").as_deref(), Some("route-3"));

    let (stdout, code) = run(
        &server,
        &sandbox.auth_file(),
        &[
            "trip",
            "leg",
            TRIP,
            "--stop",
            CUSTOM_STOP,
            "--flight",
            "195",
            "--note",
            "BR198 TPE-NRT",
        ],
    );
    assert_eq!(code, 0, "{stdout}");
    let write = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/TravelScheduleDetail/SetFlightRoute")
        .expect("SetFlightRoute was called");
    assert_eq!(write.field("Duration").as_deref(), Some("195"));
    assert_eq!(write.field("Note").as_deref(), Some("BR198 TPE-NRT"));
}

#[test]
fn trip_leg_rejects_conflicting_setters_before_any_request() {
    let server = trip_server(|_, _| None);
    let sandbox = Sandbox::new("leg-usage");
    sandbox.write_auth(MEMBER_TOKEN, "refresh-1");

    for args in [
        vec![
            "trip", "leg", TRIP, "--stop", STOP, "--route", "r-1", "--custom", "5",
        ],
        vec![
            "trip", "leg", TRIP, "--stop", STOP, "--custom", "5", "--flight", "5",
        ],
        vec!["trip", "leg", TRIP, "--stop", STOP, "--note", "x"],
        vec!["trip", "leg", TRIP, "--stop", STOP, "--mode", "bus"],
    ] {
        let (stdout, code) = run(&server, &sandbox.auth_file(), &args);
        assert_eq!(code, 2, "{args:?} {stdout}");
        assert!(stdout.starts_with("error: usage\n"), "{stdout}");
    }
    assert!(server.requests().is_empty(), "usage errors never call out");
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
fn setup_skill_writes_the_committed_path_and_is_idempotent() {
    let sandbox = Sandbox::new("skill");
    let project = sandbox.project();

    let (stdout, code) = run_in(&sandbox, &project, &[], &["setup", "skill"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.starts_with("file: \"skills/chictrip-axi/SKILL.md\"\nstatus: written\n"),
        "{stdout}"
    );
    let written = std::fs::read_to_string(project.join("skills/chictrip-axi/SKILL.md")).unwrap();
    assert!(
        written.starts_with("---\nname: chictrip-axi\n"),
        "{written}"
    );

    let (stdout, code) = run_in(&sandbox, &project, &[], &["setup", "skill"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: unchanged\n"), "{stdout}");

    let (stdout, code) = run_in(&sandbox, &project, &[], &["setup", "skill", "--check"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: current\n"), "{stdout}");

    let (stdout, code) = run_in(
        &sandbox,
        &project,
        &[],
        &["setup", "skill", "--out", "custom/SKILL.md"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        std::fs::read_to_string(project.join("custom/SKILL.md")).unwrap(),
        written
    );
}

#[test]
fn setup_skill_check_fails_when_the_file_is_missing_or_edited() {
    let sandbox = Sandbox::new("skill-check");
    let project = sandbox.project();

    let (stdout, code) = run_in(&sandbox, &project, &[], &["setup", "skill", "--check"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: not_found\n"), "{stdout}");
    assert!(stdout.contains("skills/chictrip-axi/SKILL.md"), "{stdout}");

    let (stdout, code) = run_in(&sandbox, &project, &[], &["setup", "skill"]);
    assert_eq!(code, 0, "{stdout}");

    let path = project.join("skills/chictrip-axi/SKILL.md");
    let edited = format!("{}hand edited\n", std::fs::read_to_string(&path).unwrap());
    std::fs::write(&path, &edited).unwrap();

    let (stdout, code) = run_in(&sandbox, &project, &[], &["setup", "skill", "--check"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: conflict\n"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        edited,
        "--check never writes"
    );
}

fn settings_of(project: &Path) -> serde_json::Value {
    let raw =
        std::fs::read_to_string(project.join(".claude/settings.json")).expect("settings.json");
    serde_json::from_str(&raw).expect("settings.json is JSON")
}

#[test]
fn setup_hooks_installs_repairs_and_removes_only_our_session_start_hook() {
    let sandbox = Sandbox::new("hooks");
    let project = sandbox.project();
    let empty_dir = sandbox.dir.join("empty-path");
    std::fs::create_dir_all(&empty_dir).unwrap();
    let path = [("PATH", empty_dir.to_str().unwrap())];
    let exe = Path::new(env!("CARGO_BIN_EXE_chictrip-axi"))
        .canonicalize()
        .unwrap();
    let pinned = format!("{} --timeout 5", exe.display());

    let (stdout, code) = run_in(&sandbox, &project, &path, &["setup", "hooks"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.starts_with("app: \"claude-code\"\n"), "{stdout}");
    assert!(stdout.contains("event: SessionStart\n"), "{stdout}");
    assert!(
        stdout.contains(&format!("command: \"{pinned}\"\n")),
        "{stdout}"
    );
    assert!(stdout.contains("status: installed\n"), "{stdout}");

    let file = project.join(".claude/settings.json");
    let raw = std::fs::read_to_string(&file).unwrap();
    assert!(raw.ends_with("}\n"), "{raw}");
    assert!(raw.contains("\n  \"hooks\": {"), "two-space indent: {raw}");
    let hook = &settings_of(&project)["hooks"]["SessionStart"][0]["hooks"][0];
    assert_eq!(hook["type"], "command");
    assert_eq!(hook["command"], pinned);
    assert_eq!(hook["timeout"], 10);
    assert!(
        settings_of(&project)["hooks"]["SessionStart"][0]
            .get("matcher")
            .is_none(),
        "no matcher means every start reason"
    );

    let (stdout, code) = run_in(&sandbox, &project, &path, &["setup", "hooks"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: unchanged\n"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        raw,
        "a no-op writes nothing"
    );

    std::fs::write(
        &file,
        concat!(
            "{\n",
            "  \"permissions\": {\"allow\": [\"Bash(ls:*)\"]},\n",
            "  \"hooks\": {\n",
            "    \"SessionStart\": [\n",
            "      {\"hooks\": [{\"type\": \"command\", \"command\": \"echo hi\"}]},\n",
            "      {\"hooks\": [{\"type\": \"command\", \"command\": \"/old/chictrip-axi --timeout 5\", \"timeout\": 10}]}\n",
            "    ]\n",
            "  }\n",
            "}\n"
        ),
    )
    .unwrap();
    let (stdout, code) = run_in(&sandbox, &project, &path, &["setup", "hooks"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: updated\n"), "{stdout}");
    let settings = settings_of(&project);
    assert_eq!(settings["permissions"]["allow"][0], "Bash(ls:*)");
    assert_eq!(
        settings["hooks"]["SessionStart"][0]["hooks"][0]["command"],
        "echo hi"
    );
    assert_eq!(
        settings["hooks"]["SessionStart"][1]["hooks"][0]["command"],
        pinned
    );

    let (stdout, code) = run_in(&sandbox, &project, &path, &["setup", "hooks", "--remove"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: removed\n"), "{stdout}");
    assert!(!stdout.contains("command:"), "{stdout}");
    let settings = settings_of(&project);
    assert_eq!(
        settings["hooks"]["SessionStart"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        settings["hooks"]["SessionStart"][0]["hooks"][0]["command"],
        "echo hi"
    );

    let (stdout, code) = run_in(&sandbox, &project, &path, &["setup", "hooks", "--remove"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("status: absent\n"), "{stdout}");
}

#[test]
fn setup_hooks_uses_the_bare_name_when_path_resolves_to_this_binary() {
    let sandbox = Sandbox::new("hooks-path");
    let project = sandbox.project();
    let bin = Path::new(env!("CARGO_BIN_EXE_chictrip-axi"))
        .parent()
        .unwrap()
        .to_str()
        .unwrap();

    let (stdout, code) = run_in(&sandbox, &project, &[("PATH", bin)], &["setup", "hooks"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("command: \"chictrip-axi --timeout 5\"\n"),
        "{stdout}"
    );
}

#[test]
fn setup_hooks_user_edits_the_claude_config_directory() {
    let sandbox = Sandbox::new("hooks-user");
    let project = sandbox.project();
    let empty_dir = sandbox.dir.join("empty-path");
    std::fs::create_dir_all(&empty_dir).unwrap();
    let path = empty_dir.to_str().unwrap().to_string();

    let (stdout, code) = run_in(
        &sandbox,
        &project,
        &[("PATH", path.as_str())],
        &["setup", "hooks", "--user"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("file: ~/.claude/settings.json\n"),
        "{stdout}"
    );
    assert!(stdout.contains("--remove --user"), "{stdout}");
    assert!(sandbox.dir.join(".claude/settings.json").exists());

    let config = sandbox.dir.join("cfg");
    let (stdout, code) = run_in(
        &sandbox,
        &project,
        &[
            ("PATH", path.as_str()),
            ("CLAUDE_CONFIG_DIR", config.to_str().unwrap()),
        ],
        &["setup", "hooks", "--user"],
    );
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("file: ~/cfg/settings.json\n"), "{stdout}");
    assert!(config.join("settings.json").exists());
}

#[test]
fn setup_hooks_refuses_a_settings_file_it_cannot_parse() {
    let sandbox = Sandbox::new("hooks-broken");
    let project = sandbox.project();
    let empty_dir = sandbox.dir.join("empty-path");
    std::fs::create_dir_all(&empty_dir).unwrap();
    let file = project.join(".claude/settings.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "{ not json").unwrap();

    let (stdout, code) = run_in(
        &sandbox,
        &project,
        &[("PATH", empty_dir.to_str().unwrap())],
        &["setup", "hooks"],
    );
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.starts_with("error: conflict\n"), "{stdout}");
    assert!(stdout.contains("settings.json"), "{stdout}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ not json");
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
