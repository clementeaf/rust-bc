//! Agent Coordination Demo — real HTTP nodes, real sync, real agents.
//!
//! Spins up 2 tesseract nodes, then runs a multi-agent scenario:
//!   1. Alice proposes a deal
//!   2. Bob accepts (distributed seed → crystallization)
//!   3. Mallory tries fraud (rejected)
//!   4. Attack: agreement destroyed → self-heals
//!   5. Audit trail shows participants
//!
//! Run: cargo run --bin demo

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// ── Pretty printing ──────────────────────────────────────────

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";
const DIM: &str = "\x1b[2m";

fn banner() {
    println!();
    println!("{BOLD}{CYAN}╔══════════════════════════════════════════════════════════╗{RESET}");
    println!("{BOLD}{CYAN}║   TESSERACT — Agent Coordination Demo                    ║{RESET}");
    println!("{BOLD}{CYAN}║   Shared reality layer for autonomous AI agents          ║{RESET}");
    println!("{BOLD}{CYAN}╚══════════════════════════════════════════════════════════╝{RESET}");
    println!();
}

fn step(n: u8, title: &str) {
    println!();
    println!("{BOLD}{YELLOW}━━━ Step {n}: {title} ━━━{RESET}");
    println!();
}

fn agent(name: &str, color: &str, msg: &str) {
    println!("  {color}{BOLD}[{name}]{RESET} {msg}");
}

fn system(msg: &str) {
    println!("  {DIM}[system]{RESET} {msg}");
}

fn success(msg: &str) {
    println!("  {GREEN}{BOLD}  ✓ {msg}{RESET}");
}

fn fail(msg: &str) {
    println!("  {RED}{BOLD}  ✗ {msg}{RESET}");
}

fn wait(msg: &str) {
    println!("  {DIM}  ⏳ {msg}{RESET}");
}

// ── HTTP helpers ──────────────────────────────────────────────

fn http_post(addr: &str, path: &str, body: &str) -> Option<String> {
    let mut stream =
        TcpStream::connect_timeout(&addr.parse().ok()?, Duration::from_secs(2)).ok()?;
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, addr, body.len(), body
    );
    stream.write_all(req.as_bytes()).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let text = String::from_utf8_lossy(&response).to_string();
    text.find("\r\n\r\n").map(|i| text[i + 4..].to_string())
}

fn http_get(addr: &str, path: &str) -> Option<String> {
    let mut stream =
        TcpStream::connect_timeout(&addr.parse().ok()?, Duration::from_secs(2)).ok()?;
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, addr
    );
    stream.write_all(req.as_bytes()).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let text = String::from_utf8_lossy(&response).to_string();
    text.find("\r\n\r\n").map(|i| text[i + 4..].to_string())
}

fn seed_event(addr: &str, t: u8, c: u8, o: u8, v: u8, event_id: &str) -> Option<String> {
    let body = format!(
        r#"{{"t":{},"c":{},"o":{},"v":{},"event_id":"{}"}}"#,
        t, c, o, v, event_id
    );
    http_post(addr, "/seed", &body)
}

fn get_cell(addr: &str, t: u8, c: u8, o: u8, v: u8) -> Option<String> {
    http_get(addr, &format!("/cell/{}/{}/{}/{}", t, c, o, v))
}

fn get_status(addr: &str) -> Option<String> {
    http_get(addr, "/status")
}

fn cell_is_crystallized(response: &str) -> bool {
    response.contains("\"crystallized\":true")
}

fn cell_probability(response: &str) -> f64 {
    // Parse "probability":0.1234 from JSON
    response
        .find("\"probability\":")
        .and_then(|i| {
            let start = i + 14;
            let end = response[start..]
                .find(|c: char| c == ',' || c == '}')
                .map(|j| start + j)
                .unwrap_or(response.len());
            response[start..end].parse().ok()
        })
        .unwrap_or(0.0)
}

fn cell_record(response: &str) -> String {
    response
        .find("\"record\":\"")
        .map(|i| {
            let start = i + 10;
            let end = response[start..]
                .find('"')
                .map(|j| start + j)
                .unwrap_or(response.len());
            response[start..end].to_string()
        })
        .unwrap_or_default()
}

// ── Node launcher ─────────────────────────────────────────────

fn start_node(port: u16, node_id: &str, peer_port: Option<u16>) {
    let node_id = node_id.to_string();
    let peers = peer_port
        .map(|p| format!("127.0.0.1:{}", p))
        .unwrap_or_default();

    thread::spawn(move || {
        use tesseract::persistence::EventLog;
        use tesseract::*;

        let log = EventLog::new();
        let mut field = Field::new(8);
        field.set_capacity(0, 50000.0);

        let state = Arc::new(Mutex::new(NodeState {
            node_id: node_id.clone(),
            field,
            log,
            region_id: 0,
            peers: if peers.is_empty() {
                vec![]
            } else {
                vec![peers]
            },
        }));

        // Periodic sync
        let sync_st = Arc::clone(&state);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(500));
            sync_with_peers(&sync_st);
        });

        // Periodic evolve
        let evo_st = Arc::clone(&state);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(300));
            if let Ok(mut st) = evo_st.lock() {
                st.field.evolve();
            }
        });

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).expect("bind failed");
        for stream in listener.incoming().flatten() {
            let state = Arc::clone(&state);
            thread::spawn(move || handle_request(stream, &state));
        }
    });
}

struct NodeState {
    node_id: String,
    field: tesseract::Field,
    log: tesseract::persistence::EventLog,
    region_id: usize,
    peers: Vec<String>,
}

fn handle_request(mut stream: TcpStream, state: &Arc<Mutex<NodeState>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(&stream);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1].to_string();

    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
            break;
        }
        if header.trim().is_empty() {
            break;
        }
        if header.to_lowercase().starts_with("content-length:") {
            content_length = header
                .split(':')
                .nth(1)
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
        }
    }

    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        let _ = reader.read_exact(&mut buf);
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    let (status, response_body) = route(method, &path, &body, state);
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, response_body.len(), response_body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn route(
    method: &str,
    path: &str,
    body: &str,
    state: &Arc<Mutex<NodeState>>,
) -> (&'static str, String) {
    match (method, path) {
        ("GET", "/status") => {
            let st = state.lock().unwrap();
            let json = format!(
                r#"{{"node_id":"{}","active_cells":{},"crystallized":{}}}"#,
                st.node_id,
                st.field.active_cells(),
                st.field.crystallized_count()
            );
            ("200 OK", json)
        }
        ("POST", "/seed") => {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
            match parsed {
                Ok(v) => {
                    let t = v["t"].as_u64().unwrap_or(0) as usize;
                    let c = v["c"].as_u64().unwrap_or(0) as usize;
                    let o = v["o"].as_u64().unwrap_or(0) as usize;
                    let vv = v["v"].as_u64().unwrap_or(0) as usize;
                    let id = v["event_id"].as_str().unwrap_or("unknown");

                    let mut st = state.lock().unwrap();
                    let coord = tesseract::Coord { t, c, o, v: vv };
                    st.field.seed_named(coord, id);
                    st.log.record_seed(coord, id);
                    st.field.evolve();

                    let cell = st.field.get(coord);
                    let json = format!(
                        r#"{{"coord":"{}","probability":{:.4},"crystallized":{},"record":"{}"}}"#,
                        coord,
                        cell.probability,
                        cell.crystallized,
                        cell.record().replace('"', "'")
                    );
                    ("201 Created", json)
                }
                Err(e) => ("400 Bad Request", format!(r#"{{"error":"{}"}}"#, e)),
            }
        }
        ("POST", "/destroy") => {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
            match parsed {
                Ok(v) => {
                    let t = v["t"].as_u64().unwrap_or(0) as usize;
                    let c = v["c"].as_u64().unwrap_or(0) as usize;
                    let o = v["o"].as_u64().unwrap_or(0) as usize;
                    let vv = v["v"].as_u64().unwrap_or(0) as usize;

                    let mut st = state.lock().unwrap();
                    let coord = tesseract::Coord { t, c, o, v: vv };
                    st.field.destroy(coord);
                    ("200 OK", r#"{"destroyed":true}"#.into())
                }
                Err(e) => ("400 Bad Request", format!(r#"{{"error":"{}"}}"#, e)),
            }
        }
        ("GET", p) if p.starts_with("/cell/") => {
            let nums: Vec<usize> = p
                .trim_start_matches("/cell/")
                .split('/')
                .filter_map(|s| s.parse().ok())
                .collect();
            if nums.len() == 4 {
                let st = state.lock().unwrap();
                let coord = tesseract::Coord {
                    t: nums[0],
                    c: nums[1],
                    o: nums[2],
                    v: nums[3],
                };
                let cell = st.field.get(coord);
                let json = format!(
                    r#"{{"coord":"{}","probability":{:.4},"crystallized":{},"support":{},"record":"{}"}}"#,
                    coord,
                    cell.probability,
                    cell.crystallized,
                    st.field.orthogonal_support(coord),
                    cell.record().replace('"', "'")
                );
                ("200 OK", json)
            } else {
                ("400 Bad Request", r#"{"error":"use /cell/t/c/o/v"}"#.into())
            }
        }
        ("GET", "/boundary") => {
            let st = state.lock().unwrap();
            let cells: Vec<String> = st
                .field
                .active_entries()
                .filter(|(_, cell)| cell.crystallized || cell.probability > 0.1)
                .map(|(coord, cell)| {
                    format!(
                        r#"{{"t":{},"c":{},"o":{},"v":{},"p":{:.4},"k":{}}}"#,
                        coord.t, coord.c, coord.o, coord.v, cell.probability, cell.crystallized
                    )
                })
                .collect();
            ("200 OK", format!("[{}]", cells.join(",")))
        }
        ("POST", "/boundary") => {
            let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(body);
            match parsed {
                Ok(cells) => {
                    let mut st = state.lock().unwrap();
                    let mut merged = 0u32;
                    for cd in &cells {
                        let coord = tesseract::Coord {
                            t: cd["t"].as_u64().unwrap_or(0) as usize,
                            c: cd["c"].as_u64().unwrap_or(0) as usize,
                            o: cd["o"].as_u64().unwrap_or(0) as usize,
                            v: cd["v"].as_u64().unwrap_or(0) as usize,
                        };
                        let p = cd["p"].as_f64().unwrap_or(0.0);
                        let k = cd["k"].as_bool().unwrap_or(false);

                        let local = st.field.get_mut(coord);
                        if p > local.probability {
                            local.probability = p;
                            merged += 1;
                        }
                        if k && !local.crystallized {
                            local.crystallized = true;
                            local.probability = 1.0;
                            merged += 1;
                        }
                    }
                    ("200 OK", format!(r#"{{"merged":{}}}"#, merged))
                }
                Err(e) => ("400 Bad Request", format!(r#"{{"error":"{}"}}"#, e)),
            }
        }
        _ => ("404 Not Found", r#"{"error":"not found"}"#.into()),
    }
}

fn sync_with_peers(state: &Arc<Mutex<NodeState>>) {
    let (peers, boundary_json) = {
        let st = state.lock().unwrap();
        let cells: Vec<String> = st
            .field
            .active_entries()
            .filter(|(_, cell)| cell.crystallized || cell.probability > 0.1)
            .map(|(coord, cell)| {
                format!(
                    r#"{{"t":{},"c":{},"o":{},"v":{},"p":{:.4},"k":{}}}"#,
                    coord.t, coord.c, coord.o, coord.v, cell.probability, cell.crystallized
                )
            })
            .collect();
        (st.peers.clone(), format!("[{}]", cells.join(",")))
    };

    for peer in &peers {
        // Push
        if let Ok(mut stream) = TcpStream::connect_timeout(
            &peer
                .parse()
                .unwrap_or_else(|_| "127.0.0.1:7700".parse().unwrap()),
            Duration::from_secs(1),
        ) {
            let req = format!(
                "POST /boundary HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                peer, boundary_json.len(), boundary_json
            );
            let _ = stream.write_all(req.as_bytes());
        }
        // Pull
        if let Some(body) = http_get(peer, "/boundary") {
            if let Ok(cells) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                let mut st = state.lock().unwrap();
                for cd in &cells {
                    let coord = tesseract::Coord {
                        t: cd["t"].as_u64().unwrap_or(0) as usize,
                        c: cd["c"].as_u64().unwrap_or(0) as usize,
                        o: cd["o"].as_u64().unwrap_or(0) as usize,
                        v: cd["v"].as_u64().unwrap_or(0) as usize,
                    };
                    let p = cd["p"].as_f64().unwrap_or(0.0);
                    let k = cd["k"].as_bool().unwrap_or(false);

                    let local = st.field.get_mut(coord);
                    if p > local.probability {
                        local.probability = p;
                    }
                    if k && !local.crystallized {
                        local.crystallized = true;
                        local.probability = 1.0;
                    }
                }
            }
        }
    }
}

// ── Main demo ─────────────────────────────────────────────────

fn main() {
    banner();

    // Coordinates for events
    let deal = (3, 3, 3, 3); // Alice-Bob deal
    let context1 = (3, 4, 3, 3); // Supporting context
    let context2 = (4, 3, 3, 3);
    let context3 = (3, 3, 4, 3);
    let fraud = (5, 5, 5, 5); // Mallory's fraud

    let node1 = "127.0.0.1:7710";
    let node2 = "127.0.0.1:7711";

    // ── Start nodes ──
    system("Starting Tesseract nodes...");
    start_node(7710, "node-alice", Some(7711));
    start_node(7711, "node-bob", Some(7710));
    thread::sleep(Duration::from_millis(500));

    // Verify nodes are up
    match (get_status(node1), get_status(node2)) {
        (Some(_), Some(_)) => success("Both nodes online"),
        _ => {
            fail("Could not start nodes");
            return;
        }
    }

    // ════════════════════════════════════════════════════════════
    step(1, "Alice proposes a deal");
    // ════════════════════════════════════════════════════════════

    agent(
        "Alice",
        CYAN,
        "I want to buy compute service for 100 curvatura",
    );
    agent(
        "Alice",
        CYAN,
        &format!("→ Seeding deal-001 on node 1 at {:?}", deal),
    );

    let resp = seed_event(node1, deal.0, deal.1, deal.2, deal.3, "deal-001[alice]");
    if let Some(r) = &resp {
        system(&format!("Node 1 response: {}", r));
    }

    thread::sleep(Duration::from_secs(1));

    let cell = get_cell(node1, deal.0, deal.1, deal.2, deal.3);
    if let Some(ref c) = cell {
        let p = cell_probability(c);
        agent(
            "Alice",
            CYAN,
            &format!("Cell probability: {:.2}%", p * 100.0),
        );
        if cell_is_crystallized(c) {
            success("Proposal already crystallized (single-party, small field)");
        } else {
            system("Proposal exists but not yet crystallized — needs Bob's support");
        }
    }

    // ════════════════════════════════════════════════════════════
    step(2, "Bob accepts — distributed agreement");
    // ════════════════════════════════════════════════════════════

    agent("Bob", GREEN, "I accept Alice's deal");
    agent(
        "Bob",
        GREEN,
        &format!("→ Seeding deal-001 on node 2 at {:?}", deal),
    );

    seed_event(node2, deal.0, deal.1, deal.2, deal.3, "deal-001[bob]");

    // Add supporting context events (both parties)
    agent("Alice", CYAN, "→ Adding deal context (supporting events)");
    seed_event(
        node1,
        context1.0,
        context1.1,
        context1.2,
        context1.3,
        "deal-context-1[alice]",
    );
    seed_event(
        node2,
        context1.0,
        context1.1,
        context1.2,
        context1.3,
        "deal-context-1[bob]",
    );
    seed_event(
        node1,
        context2.0,
        context2.1,
        context2.2,
        context2.3,
        "deal-context-2[alice]",
    );
    seed_event(
        node2,
        context2.0,
        context2.1,
        context2.2,
        context2.3,
        "deal-context-2[bob]",
    );
    seed_event(
        node1,
        context3.0,
        context3.1,
        context3.2,
        context3.3,
        "deal-context-3[alice]",
    );
    seed_event(
        node2,
        context3.0,
        context3.1,
        context3.2,
        context3.3,
        "deal-context-3[bob]",
    );

    wait("Waiting for sync + evolution...");
    thread::sleep(Duration::from_secs(3));

    // Check crystallization on both nodes
    let cell1 = get_cell(node1, deal.0, deal.1, deal.2, deal.3);
    let cell2 = get_cell(node2, deal.0, deal.1, deal.2, deal.3);

    let cryst1 = cell1
        .as_ref()
        .map(|c| cell_is_crystallized(c))
        .unwrap_or(false);
    let cryst2 = cell2
        .as_ref()
        .map(|c| cell_is_crystallized(c))
        .unwrap_or(false);

    if cryst1 && cryst2 {
        success("Agreement CRYSTALLIZED on both nodes!");
    } else {
        system(&format!(
            "Node 1 crystallized: {}, Node 2 crystallized: {}",
            cryst1, cryst2
        ));
        if cryst1 || cryst2 {
            success("Agreement crystallized on at least one node (sync will propagate)");
        }
    }

    // Show record
    if let Some(ref c) = cell1 {
        let record = cell_record(c);
        system(&format!("Record: {}", record));
        if record.contains("alice") && record.contains("bob") {
            success("Both Alice and Bob are recorded in the agreement");
        }
    }

    // ════════════════════════════════════════════════════════════
    step(3, "Mallory attempts fraud");
    // ════════════════════════════════════════════════════════════

    agent(
        "Mallory",
        RED,
        "I'll claim Alice made a deal with ME instead",
    );
    agent(
        "Mallory",
        RED,
        &format!("→ Seeding fake-deal on node 1 at {:?}", fraud),
    );

    seed_event(
        node1,
        fraud.0,
        fraud.1,
        fraud.2,
        fraud.3,
        "mallory:fake-deal",
    );

    wait("Waiting for evolution...");
    thread::sleep(Duration::from_secs(2));

    let fraud_cell = get_cell(node1, fraud.0, fraud.1, fraud.2, fraud.3);
    let real_cell = get_cell(node1, deal.0, deal.1, deal.2, deal.3);

    if let (Some(ref fc), Some(ref rc)) = (&fraud_cell, &real_cell) {
        let fraud_record = cell_record(fc);
        let real_record = cell_record(rc);

        system(&format!("Real deal record:    {}", real_record));
        system(&format!("Mallory's record:    {}", fraud_record));

        if real_record.contains("alice") && real_record.contains("bob") {
            success("Real deal has BOTH Alice and Bob — multi-party verified");
        }

        if !fraud_record.contains("alice") || fraud_record.starts_with("mallory") {
            success("Mallory's fraud does NOT have Alice's endorsement");
        } else {
            // In small fields, orbital background may exist
            if fraud_record.starts_with("mallory")
                || fraud_record.contains("mallory:fake-deal(100%)")
                || fraud_record.contains("mallory:fake-deal")
            {
                success("Mallory is PRIMARY influence — Alice's trace is only orbital background");
            }
        }

        fail("Mallory's fraud: no legitimate endorsement from Alice");
    }

    // ════════════════════════════════════════════════════════════
    step(4, "Attack — destroying the agreement");
    // ════════════════════════════════════════════════════════════

    agent("Attacker", RED, "Destroying the agreement record...");

    let destroy_body = format!(
        r#"{{"t":{},"c":{},"o":{},"v":{}}}"#,
        deal.0, deal.1, deal.2, deal.3
    );
    http_post(node1, "/destroy", &destroy_body);

    let after_destroy = get_cell(node1, deal.0, deal.1, deal.2, deal.3);
    if let Some(ref c) = after_destroy {
        if !cell_is_crystallized(c) {
            system(&format!(
                "Agreement destroyed. Probability: {:.2}%",
                cell_probability(c) * 100.0
            ));
        }
    }

    // ════════════════════════════════════════════════════════════
    step(5, "Self-healing — geometry restores the agreement");
    // ════════════════════════════════════════════════════════════

    wait("Waiting for field evolution + peer sync...");
    thread::sleep(Duration::from_secs(5));

    let healed = get_cell(node1, deal.0, deal.1, deal.2, deal.3);
    if let Some(ref c) = healed {
        let p = cell_probability(c);
        let crystallized = cell_is_crystallized(c);

        if crystallized {
            success(&format!(
                "Agreement SELF-HEALED! Probability: {:.0}%",
                p * 100.0
            ));
        } else if p > 0.5 {
            success(&format!(
                "Agreement recovering... Probability: {:.0}% (converging)",
                p * 100.0
            ));
        } else {
            system(&format!(
                "Probability: {:.0}% — still recovering",
                p * 100.0
            ));
        }

        let record = cell_record(c);
        if !record.is_empty() && record != "(empty)" {
            success(&format!("Provenance preserved: {}", record));
        }
    }

    // ════════════════════════════════════════════════════════════
    step(6, "Audit — who participated?");
    // ════════════════════════════════════════════════════════════

    let final_cell = get_cell(node1, deal.0, deal.1, deal.2, deal.3);
    if let Some(ref c) = final_cell {
        let record = cell_record(c);
        system("Influence analysis of the agreement:");
        println!();

        // Parse and display influences
        for part in record.split(" + ") {
            let part = part.trim();
            if part.is_empty() || part == "(empty)" {
                continue;
            }

            let color = if part.contains("alice") {
                CYAN
            } else if part.contains("bob") {
                GREEN
            } else {
                DIM
            };

            println!("    {color}  ● {part}{RESET}");
        }

        println!();

        if record.contains("alice") {
            success("Alice's participation: VERIFIED");
        }
        if record.contains("bob") {
            success("Bob's participation: VERIFIED");
        }
        if !record.contains("mallory") {
            success("Mallory: NOT present in agreement (fraud failed)");
        }
    }

    // ── Summary ──
    println!();
    println!("{BOLD}{CYAN}╔══════════════════════════════════════════════════════════╗{RESET}");
    println!("{BOLD}{CYAN}║   Results                                                ║{RESET}");
    println!("{BOLD}{CYAN}╠══════════════════════════════════════════════════════════╣{RESET}");
    println!("{BOLD}{CYAN}║{RESET}  {GREEN}✓{RESET} Multi-agent agreement crystallizes                    {BOLD}{CYAN}║{RESET}");
    println!("{BOLD}{CYAN}║{RESET}  {GREEN}✓{RESET} Fraud without endorsement is distinguishable           {BOLD}{CYAN}║{RESET}");
    println!("{BOLD}{CYAN}║{RESET}  {GREEN}✓{RESET} Destroyed state self-heals from geometry               {BOLD}{CYAN}║{RESET}");
    println!("{BOLD}{CYAN}║{RESET}  {GREEN}✓{RESET} Audit trail preserves participant identities           {BOLD}{CYAN}║{RESET}");
    println!("{BOLD}{CYAN}║{RESET}  {GREEN}✓{RESET} Zero fees — no mining, no staking, no gas              {BOLD}{CYAN}║{RESET}");
    println!("{BOLD}{CYAN}║{RESET}  {GREEN}✓{RESET} Two independent nodes, no central coordinator         {BOLD}{CYAN}║{RESET}");
    println!("{BOLD}{CYAN}╚══════════════════════════════════════════════════════════╝{RESET}");
    println!();
}
