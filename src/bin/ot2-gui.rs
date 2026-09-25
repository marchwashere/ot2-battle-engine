#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

//! Local inspection shell. The battle and all RNG logic stay in the library.
use ot2_battle_engine::{
    Battle, BattleConfig, CrtRng, ItemCatalog, Outcome, SimError, routine::ThroneOpeningRoutine,
};
use serde_json::{Value, json};
use std::{
    env,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    time::Duration,
};

const HTML: &str = include_str!("../../viewer/index.html");
const CSS: &str = include_str!("../../viewer/viewer.css");
const JS: &str = include_str!("../../viewer/viewer.js");

fn parse_seed(text: &str) -> Result<u32, SimError> {
    let text = text.trim();
    let (digits, radix) =
        if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (hex, 16)
        } else {
            (text, 10)
        };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii() && c.is_digit(radix)) {
        return Err(SimError(
            "Enter a decimal seed or 0x-prefixed hexadecimal seed.".into(),
        ));
    }
    u32::from_str_radix(digits, radix)
        .map_err(|_| SimError("Seed must be between 0 and 4294967295 (0xFFFFFFFF).".into()))
}

fn simulate(seed: u32) -> Result<Value, SimError> {
    let mut config = BattleConfig::from_json(include_str!("../../data/throne_opening.json"))?;
    config.seed = seed;
    let actors: Vec<_> = config.players.iter().map(|p| {
        json!({"name":p.name,"side":"player","max_hp":p.stats.max_hp,"max_sp":p.stats.max_sp})
    }).chain(config.enemies.iter().enumerate().map(|(i,e)| {
        json!({"name":format!("Enemy {} · {}",i+1,e.name),"side":"enemy","max_hp":e.stats.max_hp,"max_sp":e.stats.max_sp})
    })).collect();
    let mut battle = Battle::new_opening(config, ItemCatalog::builtin()?, true)?;
    let initial_state = battle.state().clone();
    let mut routine = ThroneOpeningRoutine::default();
    let mut steps = Vec::new();
    while battle.state().outcome == Outcome::Ongoing {
        if steps.len() == 16 {
            return Err(SimError("Routine exceeded its action budget.".into()));
        }
        let round = battle.state().round;
        let report = routine.step(&mut battle)?;
        steps.push(json!({"round":round,"report":report,"actors_after":battle.state().actors}));
    }
    Ok(json!({
        "initial_seed":seed, "actors":actors, "initial_state":initial_state,
        "steps":steps, "routine":routine, "state":battle.state()
    }))
}

#[derive(Clone, Copy)]
struct CrtSetup {
    seed: u32,
    advances: u64,
}
impl CrtSetup {
    fn parse(query: &str) -> Result<Self, SimError> {
        let mut seed = None;
        let mut advances = None;
        for part in query.split('&') {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| SimError("Invalid CRT setup.".into()))?;
            match key {
                "seed" if seed.is_none() => seed = Some(parse_seed(value)?),
                "advances" if advances.is_none() => {
                    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(SimError(
                            "Advances must be a nonnegative whole number.".into(),
                        ));
                    }
                    advances = Some(value.parse::<u64>().map_err(|_| {
                        SimError("Advance count exceeds 18446744073709551615.".into())
                    })?);
                }
                _ => return Err(SimError("Unknown or repeated CRT setup parameter.".into())),
            }
        }
        Ok(Self {
            seed: seed.ok_or_else(|| SimError("Initial seed is required.".into()))?,
            advances: advances.unwrap_or(0),
        })
    }
    fn battle_seed(self) -> u32 {
        let mut rng = CrtRng::new(self.seed);
        rng.advance(self.advances);
        rng.state
    }
    fn json(self) -> Value {
        // Decimal strings keep u64 counts exact in browser JavaScript and exports.
        json!({"initial_seed":self.seed,"advances":self.advances.to_string(),"battle_seed":self.battle_seed()})
    }
    fn simulate(self) -> Result<Value, SimError> {
        let mut result = simulate(self.battle_seed())?;
        result["crt_setup"] = self.json();
        Ok(result)
    }
}

struct Response {
    status: u16,
    mime: &'static str,
    body: String,
    stop: bool,
    filename: Option<String>,
}
impl Response {
    fn new(status: u16, mime: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            mime,
            body: body.into(),
            stop: false,
            filename: None,
        }
    }
    fn json(status: u16, value: Value) -> Self {
        Self::new(status, "application/json; charset=utf-8", value.to_string())
    }
    fn error(status: u16, message: &str) -> Self {
        Self::json(status, json!({"error":message}))
    }
}

fn route(request: &str, address: &str) -> Response {
    let mut lines = request.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let (Some(method), Some(path), Some(version)) = (first.next(), first.next(), first.next())
    else {
        return Response::error(400, "Invalid HTTP request.");
    };
    if first.next().is_some() || !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Response::error(400, "Invalid HTTP request.");
    }
    let mut host = None;
    let mut origin = None;
    let mut viewer_header = false;
    let mut cross_site = false;
    for line in lines.take_while(|line| !line.is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            return Response::error(400, "Invalid header.");
        };
        let value = value.trim();
        if key.eq_ignore_ascii_case("host") {
            host = Some(value);
        }
        if key.eq_ignore_ascii_case("origin") {
            origin = Some(value);
        }
        if key.eq_ignore_ascii_case("x-ot2-viewer") {
            viewer_header = value == "1";
        }
        if key.eq_ignore_ascii_case("sec-fetch-site") {
            cross_site = value == "cross-site";
        }
    }
    let expected_origin = format!("http://{address}");
    if host != Some(address) || origin.is_some_and(|v| v != expected_origin) || cross_site {
        return Response::error(403, "Open this viewer at its local address.");
    }
    match (method, path) {
        ("GET", "/") => Response::new(200, "text/html; charset=utf-8", HTML),
        ("GET", "/viewer.css") => Response::new(200, "text/css; charset=utf-8", CSS),
        ("GET", "/viewer.js") => Response::new(200, "text/javascript; charset=utf-8", JS),
        ("GET", "/api/health") => Response::json(200, json!({"app":"OT2 Battle Viewer"})),
        ("GET", path) if path.starts_with("/api/crt?") => {
            match CrtSetup::parse(&path["/api/crt?".len()..]) {
                Err(e) => Response::error(400, &e.to_string()),
                Ok(setup) => Response::json(200, setup.json()),
            }
        }
        ("GET", path) if path.starts_with("/api/simulate?") => {
            match CrtSetup::parse(&path["/api/simulate?".len()..]) {
                Err(e) => Response::error(400, &e.to_string()),
                Ok(setup) => match setup.simulate() {
                    Ok(value) => Response::json(200, value),
                    Err(e) => Response::error(422, &e.to_string()),
                },
            }
        }
        ("GET", path) if path.starts_with("/api/export?") => {
            match CrtSetup::parse(&path["/api/export?".len()..]) {
                Err(e) => Response::error(400, &e.to_string()),
                Ok(setup) => match setup.simulate() {
                    Ok(value) => {
                        let mut response = Response::new(
                            200,
                            "application/json; charset=utf-8",
                            serde_json::to_string_pretty(&value).expect("serializable result"),
                        );
                        response.filename = Some(if setup.advances == 0 {
                            format!("ot2-throne-seed-{}.json", setup.seed)
                        } else {
                            format!(
                                "ot2-throne-seed-{}-advances-{}.json",
                                setup.seed, setup.advances
                            )
                        });
                        response
                    }
                    Err(e) => Response::error(422, &e.to_string()),
                },
            }
        }
        ("POST", "/api/shutdown") if viewer_header => {
            let mut response = Response::json(200, json!({"closed":true}));
            response.stop = true;
            response
        }
        _ => Response::error(404, "Not found."),
    }
}

fn handle(mut stream: TcpStream, address: &str) -> std::io::Result<bool> {
    stream.set_read_timeout(Some(Duration::from_millis(750)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 2048];
    let response = loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Ok(false);
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > 8192 {
            break Response::error(431, "Request too large.");
        }
        if bytes.windows(4).any(|v| v == b"\r\n\r\n") {
            break match std::str::from_utf8(&bytes) {
                Ok(request) => route(request, address),
                Err(_) => Response::error(400, "Invalid request encoding."),
            };
        }
    };
    let phrase = match response.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    let attachment = response.filename.as_ref().map_or_else(String::new, |name| {
        format!("Content-Disposition: attachment; filename=\"{name}\"\r\n")
    });
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{attachment}Connection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n{}",
        response.status,
        phrase,
        response.mime,
        response.body.len(),
        response.body
    )?;
    Ok(response.stop)
}

fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .creation_flags(0x0800_0000)
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut port = 0;
    let mut launch = true;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-open" => launch = false,
            "--port" => {
                port = args
                    .next()
                    .ok_or("--port requires a number")?
                    .parse::<u16>()?
            }
            _ => return Err("usage: ot2-gui [--no-open] [--port 0..65535]".into()),
        }
    }
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let address = listener.local_addr()?.to_string();
    let url = format!("http://{address}/");
    println!("{url}");
    std::io::stdout().flush()?;
    if launch {
        open_browser(&url)?;
    }
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if handle(stream, &address).unwrap_or(false) {
                    break;
                }
            }
            Err(error) => eprintln!("Connection error: {error}"),
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("OT2 Battle Viewer: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_boundaries_and_invalid_inputs() {
        for (text, expected) in [
            ("0", 0),
            ("123", 123),
            ("0x7b", 123),
            ("0XFFFFFFFF", u32::MAX),
            ("4294967295", u32::MAX),
        ] {
            assert_eq!(parse_seed(text).unwrap(), expected);
        }
        for invalid in [
            "",
            "0x",
            "-1",
            "+1",
            "1.2",
            "1e3",
            "4294967296",
            "0x100000000",
            "123&x=1",
        ] {
            assert!(parse_seed(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn response_preserves_core_checkpoints_and_per_action_resources() {
        let value = simulate(123).unwrap();
        assert_eq!(
            value["state"]["rng"],
            json!({"state":3423024866u32,"draws":145})
        );
        assert_eq!(
            value["state"]["opening_crt"]["after_battle"]["encounter_steps"],
            798
        );
        let steps = value["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 6);
        assert_eq!(steps[0]["round"], 1);
        assert_eq!(steps[5]["round"], 2);
        assert_eq!(steps[5]["actors_after"], value["state"]["actors"]);
        assert_eq!(value["state"]["outcome"], "victory");
    }

    #[test]
    fn prebattle_advances_use_full_state_and_do_not_change_battle_draw_counts() {
        let setup = CrtSetup::parse("seed=0x7b&advances=2").unwrap();
        assert_eq!(setup.battle_seed(), 3_396_176_589);
        let configured = setup.simulate().unwrap();
        let direct = simulate(3_396_176_589).unwrap();
        assert_eq!(configured["state"], direct["state"]);
        assert_eq!(configured["steps"], direct["steps"]);
        assert_eq!(configured["initial_seed"], 3_396_176_589u32);
        assert_eq!(
            configured["crt_setup"],
            json!({"initial_seed":123,"advances":"2","battle_seed":3396176589u32})
        );
        let unchanged = CrtSetup::parse("seed=123&advances=0")
            .unwrap()
            .simulate()
            .unwrap();
        assert_eq!(unchanged["state"], simulate(123).unwrap()["state"]);
    }

    #[test]
    fn advance_input_preview_and_export_preserve_configuration() {
        for invalid in [
            "seed=123&advances=-1",
            "seed=123&advances=1.5",
            "seed=123&advances=",
            "seed=123&advances=18446744073709551616",
            "seed=1&seed=2",
            "seed=1&advances=0&advances=1",
        ] {
            assert!(CrtSetup::parse(invalid).is_err(), "{invalid}");
        }
        let largest = CrtSetup::parse("seed=123&advances=18446744073709551615").unwrap();
        assert_eq!(largest.json()["advances"], "18446744073709551615");
        let address = "127.0.0.1:12345";
        let request = |path: &str| format!("GET {path} HTTP/1.1\r\nHost: {address}\r\n\r\n");
        let preview = route(&request("/api/crt?seed=123&advances=2"), address);
        assert_eq!(preview.status, 200);
        assert_eq!(
            serde_json::from_str::<Value>(&preview.body).unwrap()["battle_seed"],
            3_396_176_589u32
        );
        let export = route(&request("/api/export?seed=123&advances=2"), address);
        assert_eq!(
            export.filename.as_deref(),
            Some("ot2-throne-seed-123-advances-2.json")
        );
        let value = serde_json::from_str::<Value>(&export.body).unwrap();
        assert_eq!(value["crt_setup"]["initial_seed"], 123);
        assert_eq!(value["crt_setup"]["advances"], "2");
        assert_eq!(value["initial_seed"], 3_396_176_589u32);
    }

    #[test]
    fn local_routes_validate_seeds_and_reject_cross_origin_access() {
        let address = "127.0.0.1:12345";
        let request = |path: &str| format!("GET {path} HTTP/1.1\r\nHost: {address}\r\n\r\n");
        assert_eq!(
            route(&request("/api/simulate?seed=0x7b"), address).status,
            200
        );
        assert_eq!(
            route(&request("/api/simulate?seed=4294967296"), address).status,
            400
        );
        assert_eq!(route(&request("/not-found"), address).status, 404);
        assert_eq!(
            route("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n", address).status,
            403
        );
        let foreign = format!(
            "GET /api/health HTTP/1.1\r\nHost: {address}\r\nOrigin: https://example.com\r\n\r\n"
        );
        assert_eq!(route(&foreign, address).status, 403);
        assert!(!route(&request("/api/shutdown"), address).stop);
        let export = route(&request("/api/export?seed=0x7b"), address);
        assert_eq!(export.filename.as_deref(), Some("ot2-throne-seed-123.json"));
        assert_eq!(
            serde_json::from_str::<Value>(&export.body).unwrap()["initial_seed"],
            123
        );
    }
}
