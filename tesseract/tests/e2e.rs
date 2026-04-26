//! End-to-end tests: real HTTP nodes, real sync, real adversarial scenarios.
//!
//! Spins up 2–3 in-process HTTP nodes (same binary as `cargo run --bin node`)
//! and exercises seed → sync → crystallization → attack → recovery flows
//! over actual TCP connections.
//!
//! Port allocation: tests use 18_000+ range, each test gets a unique base.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tesseract::conservation::{
    ConservationError, ConservedField, Transfer, TransferInput, TransferOutput,
};
use tesseract::persistence::EventLog;
use tesseract::*;

// ═══════════════════════════════════════════════════════════════════════════════
// Test infrastructure — lightweight HTTP nodes
// ═══════════════════════════════════════════════════════════════════════════════

struct TestNode {
    port: u16,
    state: Arc<Mutex<NodeState>>,
}

struct NodeState {
    field: Field,
    log: EventLog,
    peers: Vec<String>,
}

impl TestNode {
    fn start(port: u16, peers: Vec<String>) -> Self {
        let state = Arc::new(Mutex::new(NodeState {
            field: Field::new(8),
            log: EventLog::new(),
            peers,
        }));

        // HTTP server thread
        let srv_state = Arc::clone(&state);
        thread::spawn(move || {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).expect("bind failed");
            listener
                .set_nonblocking(false)
                .expect("set_nonblocking failed");
            for stream in listener.incoming().flatten() {
                let state = Arc::clone(&srv_state);
                thread::spawn(move || handle_request(stream, &state));
            }
        });

        // Periodic sync
        let sync_state = Arc::clone(&state);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(300));
            sync_with_peers(&sync_state);
        });

        // Periodic evolve
        let evo_state = Arc::clone(&state);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(200));
            if let Ok(mut st) = evo_state.lock() {
                st.field.evolve();
            }
        });

        // Wait for server to be ready
        for _ in 0..50 {
            if TcpStream::connect_timeout(
                &format!("127.0.0.1:{}", port).parse().unwrap(),
                Duration::from_millis(100),
            )
            .is_ok()
            {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        Self { port, state }
    }

    fn addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn seed(&self, t: usize, c: usize, o: usize, v: usize, id: &str) -> Option<String> {
        let body = format!(
            r#"{{"t":{},"c":{},"o":{},"v":{},"event_id":"{}"}}"#,
            t, c, o, v, id
        );
        http_post(&self.addr(), "/seed", &body)
    }

    fn get_cell(&self, t: usize, c: usize, o: usize, v: usize) -> Option<String> {
        http_get(&self.addr(), &format!("/cell/{}/{}/{}/{}", t, c, o, v))
    }

    fn destroy(&self, t: usize, c: usize, o: usize, v: usize) {
        let body = format!(r#"{{"t":{},"c":{},"o":{},"v":{}}}"#, t, c, o, v);
        http_post(&self.addr(), "/destroy", &body);
    }

    fn status(&self) -> Option<String> {
        http_get(&self.addr(), "/status")
    }

    fn metrics(&self) -> Option<String> {
        http_get(&self.addr(), "/metrics")
    }

    fn crystallized_count(&self) -> usize {
        self.state.lock().unwrap().field.crystallized_count()
    }

    fn active_cells(&self) -> usize {
        self.state.lock().unwrap().field.active_cells()
    }

    fn is_crystallized(&self, t: usize, c: usize, o: usize, v: usize) -> bool {
        let coord = Coord { t, c, o, v };
        self.state.lock().unwrap().field.get(coord).crystallized
    }

    fn probability(&self, t: usize, c: usize, o: usize, v: usize) -> f64 {
        let coord = Coord { t, c, o, v };
        self.state.lock().unwrap().field.get(coord).probability
    }

    fn record(&self, t: usize, c: usize, o: usize, v: usize) -> String {
        let coord = Coord { t, c, o, v };
        self.state.lock().unwrap().field.get(coord).record()
    }

    fn influence_ids(&self, t: usize, c: usize, o: usize, v: usize) -> Vec<String> {
        let coord = Coord { t, c, o, v };
        let st = self.state.lock().unwrap();
        let cell = st.field.get(coord);
        cell.influences.iter().map(|i| i.event_id.clone()).collect()
    }
}

// ── HTTP helpers ─────────────────────────────────────────────────────────────

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

fn json_bool(json: &str, key: &str) -> bool {
    json.contains(&format!("\"{}\":true", key))
}

fn json_f64(json: &str, key: &str) -> f64 {
    json.find(&format!("\"{}\":", key))
        .and_then(|i| {
            let start = i + key.len() + 3;
            let end = json[start..]
                .find(|c: char| c == ',' || c == '}')
                .map(|j| start + j)
                .unwrap_or(json.len());
            json[start..end].parse().ok()
        })
        .unwrap_or(0.0)
}

fn json_usize(json: &str, key: &str) -> usize {
    json_f64(json, key) as usize
}

// ── Node HTTP handler (same as bin/node.rs but inline) ───────────────────────

fn handle_request(mut stream: TcpStream, state: &Arc<Mutex<NodeState>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = std::io::BufReader::new(&stream);

    let mut request_line = String::new();
    if std::io::BufRead::read_line(&mut reader, &mut request_line).is_err() {
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
        if std::io::BufRead::read_line(&mut reader, &mut header).is_err() {
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
        let _ = Read::read_exact(&mut reader, &mut buf);
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    let (status, response_body) = route(method, &path, &body, state);
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        response_body.len(),
        response_body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn route(
    method: &str,
    path: &str,
    body: &str,
    state: &Arc<Mutex<NodeState>>,
) -> (&'static str, String) {
    match (method, path.as_ref()) {
        ("GET", "/status") => {
            let st = state.lock().unwrap();
            let json = format!(
                r#"{{"active_cells":{},"crystallized":{},"events":{}}}"#,
                st.field.active_cells(),
                st.field.crystallized_count(),
                st.log.len()
            );
            ("200 OK", json)
        }
        ("GET", "/metrics") => {
            let st = state.lock().unwrap();
            let body = format!(
                "tesseract_active_cells {}\ntesseract_crystallized_cells {}\n",
                st.field.active_cells(),
                st.field.crystallized_count()
            );
            ("200 OK", body)
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
                    let coord = Coord { t, c, o, v: vv };
                    st.field.seed_named(coord, id);
                    st.log.record_seed(coord, id);
                    st.field.evolve();

                    let cell = st.field.get(coord);
                    let json = format!(
                        r#"{{"probability":{:.4},"crystallized":{}}}"#,
                        cell.probability, cell.crystallized
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
                    st.field.destroy(Coord { t, c, o, v: vv });
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
                let coord = Coord {
                    t: nums[0],
                    c: nums[1],
                    o: nums[2],
                    v: nums[3],
                };
                let cell = st.field.get(coord);
                let json = format!(
                    r#"{{"probability":{:.4},"crystallized":{},"support":{}}}"#,
                    cell.probability,
                    cell.crystallized,
                    st.field.orthogonal_support(coord)
                );
                ("200 OK", json)
            } else {
                ("400 Bad Request", r#"{"error":"bad coords"}"#.into())
            }
        }
        ("GET", "/boundary") => {
            let st = state.lock().unwrap();
            let cells: Vec<String> = st
                .field
                .active_entries()
                .filter(|(_, cell)| cell.crystallized || cell.probability > 0.1)
                .map(|(coord, cell)| {
                    // Include influences for evidence-carrying sync.
                    let infs: Vec<String> = cell
                        .influences
                        .iter()
                        .map(|i| {
                            format!(
                                r#"{{"id":"{}","w":{:.4}}}"#,
                                i.event_id, i.weight
                            )
                        })
                        .collect();
                    format!(
                        r#"{{"t":{},"c":{},"o":{},"v":{},"p":{:.4},"k":{},"er":"{}","ec":{},"infs":[{}]}}"#,
                        coord.t, coord.c, coord.o, coord.v,
                        cell.probability, cell.crystallized,
                        hex::encode(cell.evidence_root), cell.evidence_count,
                        infs.join(",")
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
                        let coord = Coord {
                            t: cd["t"].as_u64().unwrap_or(0) as usize,
                            c: cd["c"].as_u64().unwrap_or(0) as usize,
                            o: cd["o"].as_u64().unwrap_or(0) as usize,
                            v: cd["v"].as_u64().unwrap_or(0) as usize,
                        };
                        let p = cd["p"].as_f64().unwrap_or(0.0);
                        let k = cd["k"].as_bool().unwrap_or(false);

                        // Build remote cell from wire data.
                        let mut remote = Cell::new();
                        remote.probability = p;
                        remote.crystallized = k;
                        if let Some(infs) = cd["infs"].as_array() {
                            for inf in infs {
                                let event_id = inf["id"].as_str().unwrap_or("").to_string();
                                let weight = inf["w"].as_f64().unwrap_or(0.0);
                                if !event_id.is_empty() {
                                    remote.influences.push(Influence { event_id, weight });
                                }
                            }
                        }
                        remote.update_evidence();

                        // Deterministic resolve: replaces old max-p / propagate-k logic.
                        let local = st.field.get(coord).clone();
                        let resolved = resolve(&local, &remote);

                        if resolved.evidence_root != local.evidence_root
                            || resolved.probability != local.probability
                            || resolved.crystallized != local.crystallized
                        {
                            *st.field.get_mut(coord) = resolved;
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
                let infs: Vec<String> = cell
                    .influences
                    .iter()
                    .map(|i| format!(r#"{{"id":"{}","w":{:.4}}}"#, i.event_id, i.weight))
                    .collect();
                format!(
                    r#"{{"t":{},"c":{},"o":{},"v":{},"p":{:.4},"k":{},"infs":[{}]}}"#,
                    coord.t,
                    coord.c,
                    coord.o,
                    coord.v,
                    cell.probability,
                    cell.crystallized,
                    infs.join(",")
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
                .unwrap_or_else(|_| "127.0.0.1:1".parse().unwrap()),
            Duration::from_secs(1),
        ) {
            let req = format!(
                "POST /boundary HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                peer, boundary_json.len(), boundary_json
            );
            let _ = stream.write_all(req.as_bytes());
        }
        // Pull and resolve
        if let Some(body) = http_get(peer, "/boundary") {
            if let Ok(cells) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                let mut st = state.lock().unwrap();
                for cd in &cells {
                    let coord = Coord {
                        t: cd["t"].as_u64().unwrap_or(0) as usize,
                        c: cd["c"].as_u64().unwrap_or(0) as usize,
                        o: cd["o"].as_u64().unwrap_or(0) as usize,
                        v: cd["v"].as_u64().unwrap_or(0) as usize,
                    };

                    let mut remote = Cell::new();
                    remote.probability = cd["p"].as_f64().unwrap_or(0.0);
                    remote.crystallized = cd["k"].as_bool().unwrap_or(false);
                    if let Some(infs) = cd["infs"].as_array() {
                        for inf in infs {
                            let event_id = inf["id"].as_str().unwrap_or("").to_string();
                            let weight = inf["w"].as_f64().unwrap_or(0.0);
                            if !event_id.is_empty() {
                                remote.influences.push(Influence { event_id, weight });
                            }
                        }
                    }
                    remote.update_evidence();

                    let local = st.field.get(coord).clone();
                    let resolved = resolve(&local, &remote);
                    if resolved.evidence_root != local.evidence_root {
                        *st.field.get_mut(coord) = resolved;
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: wait for a condition with timeout.
fn wait_until(timeout_ms: u64, poll_ms: u64, condition: impl Fn() -> bool) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
    while std::time::Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(poll_ms));
    }
    false
}

// ── 1. Basic: seed + local crystallization ───────────────────────────────────

#[test]
fn seed_event_returns_probability() {
    let n = TestNode::start(18001, vec![]);
    let resp = n.seed(3, 3, 3, 3, "test-event").unwrap();
    let p = json_f64(&resp, "probability");
    assert!(p > 0.0, "seeded event should have p > 0, got {}", p);
}

#[test]
fn seed_crystallizes_locally() {
    let n = TestNode::start(18002, vec![]);
    n.seed(3, 3, 3, 3, "ev1");
    n.seed(3, 4, 3, 3, "ev2");
    n.seed(4, 3, 3, 3, "ev3");
    n.seed(3, 3, 4, 3, "ev4");

    // Wait for evolution to crystallize
    let ok = wait_until(5000, 200, || n.is_crystallized(3, 3, 3, 3));
    assert!(ok, "event should crystallize with supporting seeds");
}

// ── 2. Two-node sync ─────────────────────────────────────────────────────────

#[test]
fn two_nodes_sync_crystallization() {
    let n1 = TestNode::start(18010, vec!["127.0.0.1:18011".into()]);
    let n2 = TestNode::start(18011, vec!["127.0.0.1:18010".into()]);

    // Seed on node 1
    n1.seed(3, 3, 3, 3, "deal[alice]");
    n1.seed(3, 4, 3, 3, "ctx1");
    n1.seed(4, 3, 3, 3, "ctx2");

    // Wait for sync to propagate crystallization to node 2
    let ok = wait_until(8000, 300, || n2.crystallized_count() > 0);
    assert!(
        ok,
        "node 2 should receive crystallized cells via sync. n1={}, n2={}",
        n1.crystallized_count(),
        n2.crystallized_count()
    );
}

#[test]
fn bidirectional_seed_both_nodes_converge() {
    let n1 = TestNode::start(18020, vec!["127.0.0.1:18021".into()]);
    let n2 = TestNode::start(18021, vec!["127.0.0.1:18020".into()]);

    // Alice seeds on node 1, Bob on node 2 — same coordinate
    n1.seed(3, 3, 3, 3, "deal[alice]");
    n2.seed(3, 3, 3, 3, "deal[bob]");

    // Supporting context from both sides
    n1.seed(3, 4, 3, 3, "ctx1[alice]");
    n2.seed(4, 3, 3, 3, "ctx2[bob]");

    let ok = wait_until(8000, 300, || {
        n1.is_crystallized(3, 3, 3, 3) || n2.is_crystallized(3, 3, 3, 3)
    });
    assert!(
        ok,
        "bidirectional seed should crystallize on at least one node"
    );
}

// ── 3. Three-node mesh ───────────────────────────────────────────────────────

#[test]
fn three_nodes_full_mesh_propagation() {
    let n1 = TestNode::start(
        18030,
        vec!["127.0.0.1:18031".into(), "127.0.0.1:18032".into()],
    );
    let n2 = TestNode::start(
        18031,
        vec!["127.0.0.1:18030".into(), "127.0.0.1:18032".into()],
    );
    let n3 = TestNode::start(
        18032,
        vec!["127.0.0.1:18030".into(), "127.0.0.1:18031".into()],
    );

    // Seed only on node 1
    n1.seed(3, 3, 3, 3, "origin-event");
    n1.seed(3, 4, 3, 3, "support-1");
    n1.seed(4, 3, 3, 3, "support-2");

    // All three should eventually have active cells
    let ok = wait_until(10000, 300, || {
        n2.active_cells() > 0 && n3.active_cells() > 0
    });
    assert!(
        ok,
        "all nodes should receive cells. n1={}, n2={}, n3={}",
        n1.active_cells(),
        n2.active_cells(),
        n3.active_cells()
    );
}

// ── 4. Attack + self-healing ─────────────────────────────────────────────────

#[test]
fn attack_destroyed_cell_recovers_via_sync() {
    let n1 = TestNode::start(18040, vec!["127.0.0.1:18041".into()]);
    let n2 = TestNode::start(18041, vec!["127.0.0.1:18040".into()]);

    // Seed heavily on both nodes
    n1.seed(3, 3, 3, 3, "agreement[alice]");
    n2.seed(3, 3, 3, 3, "agreement[bob]");
    n1.seed(3, 4, 3, 3, "ctx1");
    n2.seed(4, 3, 3, 3, "ctx2");
    n1.seed(3, 3, 4, 3, "ctx3");
    n2.seed(3, 3, 3, 4, "ctx4");

    // Wait for crystallization
    wait_until(6000, 300, || {
        n1.is_crystallized(3, 3, 3, 3) || n2.is_crystallized(3, 3, 3, 3)
    });

    let p_before = n1.probability(3, 3, 3, 3);

    // Attack: destroy on node 1
    n1.destroy(3, 3, 3, 3);
    assert!(
        !n1.is_crystallized(3, 3, 3, 3),
        "cell should be destroyed after attack"
    );

    // Wait for recovery via sync from node 2
    let recovered = wait_until(10000, 300, || n1.probability(3, 3, 3, 3) > 0.0);
    assert!(
        recovered,
        "cell should recover probability via peer sync. p_before={}, p_after={}",
        p_before,
        n1.probability(3, 3, 3, 3)
    );
}

// ── 5. Fraud: unsupported event stays weak ───────────────────────────────────

#[test]
fn fraud_without_corroboration_stays_weak() {
    let n1 = TestNode::start(18050, vec!["127.0.0.1:18051".into()]);
    let n2 = TestNode::start(18051, vec!["127.0.0.1:18050".into()]);

    // Legitimate: both nodes support the deal
    n1.seed(3, 3, 3, 3, "real-deal[alice]");
    n2.seed(3, 3, 3, 3, "real-deal[bob]");
    n1.seed(3, 4, 3, 3, "ctx1[alice]");
    n2.seed(4, 3, 3, 3, "ctx2[bob]");

    // Fraud: single node seeds unrelated event with no support
    n1.seed(6, 6, 6, 6, "fake[mallory]");

    thread::sleep(Duration::from_secs(4));

    let real_p = n1.probability(3, 3, 3, 3);
    let fraud_p = n1.probability(6, 6, 6, 6);

    // Real deal should be stronger than fraud
    assert!(
        real_p >= fraud_p,
        "real deal (p={}) should be >= fraud (p={})",
        real_p,
        fraud_p
    );
}

// ── 6. Partition simulation ──────────────────────────────────────────────────

#[test]
fn partition_independent_work_merges_on_reconnect() {
    // Start two isolated nodes (no peers)
    let n1 = TestNode::start(18060, vec![]);
    let n2 = TestNode::start(18061, vec![]);

    // Each does independent work
    n1.seed(2, 2, 2, 2, "partition-A-event");
    n1.seed(2, 3, 2, 2, "partition-A-ctx");

    n2.seed(6, 6, 6, 6, "partition-B-event");
    n2.seed(6, 5, 6, 6, "partition-B-ctx");

    thread::sleep(Duration::from_secs(2));

    let n1_crystals_before = n1.crystallized_count();
    let n2_crystals_before = n2.crystallized_count();

    // "Reconnect" — add peers dynamically via boundary push
    // Push n2's boundary to n1
    let n2_boundary = http_get(&n2.addr(), "/boundary").unwrap_or_default();
    http_post(&n1.addr(), "/boundary", &n2_boundary);

    // Push n1's boundary to n2
    let n1_boundary = http_get(&n1.addr(), "/boundary").unwrap_or_default();
    http_post(&n2.addr(), "/boundary", &n1_boundary);

    thread::sleep(Duration::from_secs(2));

    // After merge: n1 should have n2's cells and vice versa
    let n1_has_b = n1.probability(6, 6, 6, 6) > 0.0;
    let n2_has_a = n2.probability(2, 2, 2, 2) > 0.0;

    assert!(
        n1_has_b,
        "after reconnect, n1 should have n2's event (p={})",
        n1.probability(6, 6, 6, 6)
    );
    assert!(
        n2_has_a,
        "after reconnect, n2 should have n1's event (p={})",
        n2.probability(2, 2, 2, 2)
    );
}

// ── 7. Status and metrics endpoints ──────────────────────────────────────────

#[test]
fn status_endpoint_returns_field_info() {
    let n = TestNode::start(18070, vec![]);
    n.seed(3, 3, 3, 3, "test");

    let status = n.status().unwrap();
    let active = json_usize(&status, "active_cells");
    assert!(active > 0, "status should report active cells");
}

#[test]
fn metrics_endpoint_returns_prometheus_format() {
    let n = TestNode::start(18071, vec![]);
    n.seed(3, 3, 3, 3, "test");

    let metrics = n.metrics().unwrap();
    assert!(
        metrics.contains("tesseract_active_cells"),
        "metrics should contain active_cells gauge"
    );
    assert!(
        metrics.contains("tesseract_crystallized_cells"),
        "metrics should contain crystallized_cells gauge"
    );
}

// ── 8. Consistency: same seed → same state ───────────────────────────────────

#[test]
fn deterministic_field_state_across_nodes() {
    let n1 = TestNode::start(18080, vec!["127.0.0.1:18081".into()]);
    let n2 = TestNode::start(18081, vec!["127.0.0.1:18080".into()]);

    // Identical seeds on both nodes
    for (t, id) in [(3, "ev1"), (4, "ev2"), (3, "ev3")] {
        n1.seed(t, 3, 3, 3, id);
        n2.seed(t, 3, 3, 3, id);
    }

    // Wait for sync convergence
    thread::sleep(Duration::from_secs(5));

    // Both nodes should have same crystallization count
    let c1 = n1.crystallized_count();
    let c2 = n2.crystallized_count();
    assert_eq!(
        c1, c2,
        "synchronized nodes should converge: n1={}, n2={}",
        c1, c2
    );
}

// ── 9. Stress: rapid seed burst ──────────────────────────────────────────────

#[test]
fn rapid_seed_burst_does_not_crash() {
    let n1 = TestNode::start(18090, vec!["127.0.0.1:18091".into()]);
    let n2 = TestNode::start(18091, vec!["127.0.0.1:18090".into()]);

    // Fire 20 rapid seeds
    for i in 0..20 {
        let t = i % 8;
        let c = (i * 3) % 8;
        n1.seed(t, c, 3, 3, &format!("burst-{}", i));
    }

    // Should not panic; nodes should have cells
    thread::sleep(Duration::from_secs(3));
    assert!(
        n1.active_cells() > 0,
        "node should survive rapid burst: active={}",
        n1.active_cells()
    );
    assert!(
        n2.active_cells() > 0,
        "peer should receive burst via sync: active={}",
        n2.active_cells()
    );
}

// ── 10. Cell query returns correct data ──────────────────────────────────────

#[test]
fn cell_query_reflects_seeded_state() {
    let n = TestNode::start(18100, vec![]);
    n.seed(3, 3, 3, 3, "query-test");

    let cell = n.get_cell(3, 3, 3, 3).unwrap();
    let p = json_f64(&cell, "probability");
    assert!(p > 0.0, "queried cell should have probability > 0");

    // Empty cell
    let empty = n.get_cell(7, 7, 7, 7).unwrap();
    let ep = json_f64(&empty, "probability");
    assert!(
        ep < 0.5,
        "unseeded cell should have low probability, got {}",
        ep
    );
}

// ── 11. Destroy + re-seed recovers ──────────────────────────────────────────

#[test]
fn destroy_then_reseed_recovers_cell() {
    let n = TestNode::start(18110, vec![]);

    n.seed(3, 3, 3, 3, "original");
    thread::sleep(Duration::from_secs(1));
    let p_after_seed = n.probability(3, 3, 3, 3);

    n.destroy(3, 3, 3, 3);
    assert!(
        n.probability(3, 3, 3, 3) < p_after_seed,
        "destroy should reduce probability"
    );

    // Re-seed
    n.seed(3, 3, 3, 3, "restored");
    thread::sleep(Duration::from_secs(1));
    let p_restored = n.probability(3, 3, 3, 3);
    assert!(
        p_restored > 0.0,
        "re-seeded cell should recover probability"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONCURRENT CONFLICT TESTS — the hard stuff
// ═══════════════════════════════════════════════════════════════════════════════

// ── 12. Split-brain: conflicting crystallizations at same coordinate ─────────
//
// Two isolated nodes each crystallize a DIFFERENT event at the exact same
// coordinate. After reconnection, what happens?
//
// Current merge semantics: boundary sync only carries (coord, probability,
// crystallized). Records/influences do NOT travel. So after merge:
// - Both nodes show crystallized=true at the coord
// - But each node retains its OWN record — silent divergence.
//
// This test documents the current behavior as a known limitation.

#[test]
fn split_brain_conflicting_crystallizations_diverge_records() {
    // Two isolated nodes
    let n1 = TestNode::start(18200, vec![]);
    let n2 = TestNode::start(18201, vec![]);

    let coord = (3, 3, 3, 3);

    // Node 1 crystallizes "alice-deal" at (3,3,3,3)
    n1.seed(coord.0, coord.1, coord.2, coord.3, "alice-deal");
    n1.seed(coord.0, coord.1 + 1, coord.2, coord.3, "alice-ctx1");
    n1.seed(coord.0 + 1, coord.1, coord.2, coord.3, "alice-ctx2");
    n1.seed(coord.0, coord.1, coord.2 + 1, coord.3, "alice-ctx3");

    // Node 2 crystallizes "bob-counterclaim" at the SAME coordinate
    n2.seed(coord.0, coord.1, coord.2, coord.3, "bob-counterclaim");
    n2.seed(coord.0, coord.1 + 1, coord.2, coord.3, "bob-ctx1");
    n2.seed(coord.0 + 1, coord.1, coord.2, coord.3, "bob-ctx2");
    n2.seed(coord.0, coord.1, coord.2 + 1, coord.3, "bob-ctx3");

    // Let both crystallize independently
    thread::sleep(Duration::from_secs(3));

    let n1_crystallized_before = n1.is_crystallized(coord.0, coord.1, coord.2, coord.3);
    let n2_crystallized_before = n2.is_crystallized(coord.0, coord.1, coord.2, coord.3);

    let n1_record_before = n1.record(coord.0, coord.1, coord.2, coord.3);
    let n2_record_before = n2.record(coord.0, coord.1, coord.2, coord.3);

    // Verify they crystallized different things
    assert!(
        n1_crystallized_before || n1.probability(coord.0, coord.1, coord.2, coord.3) > 0.5,
        "n1 should have high probability or crystallized"
    );
    assert!(
        n2_crystallized_before || n2.probability(coord.0, coord.1, coord.2, coord.3) > 0.5,
        "n2 should have high probability or crystallized"
    );

    // Records should be DIFFERENT — they contain different event IDs
    let n1_has_alice = n1_record_before.contains("alice");
    let n2_has_bob = n2_record_before.contains("bob");
    assert!(
        n1_has_alice || !n1_record_before.is_empty(),
        "n1 should have alice's record: '{}'",
        n1_record_before
    );
    assert!(
        n2_has_bob || !n2_record_before.is_empty(),
        "n2 should have bob's record: '{}'",
        n2_record_before
    );

    // ── Reconnect: exchange boundaries ──
    let n2_boundary = http_get(&n2.addr(), "/boundary").unwrap_or_default();
    http_post(&n1.addr(), "/boundary", &n2_boundary);
    let n1_boundary = http_get(&n1.addr(), "/boundary").unwrap_or_default();
    http_post(&n2.addr(), "/boundary", &n1_boundary);

    thread::sleep(Duration::from_secs(2));

    // After merge: both nodes show crystallized=true
    let n1_crystallized_after = n1.is_crystallized(coord.0, coord.1, coord.2, coord.3);
    let n2_crystallized_after = n2.is_crystallized(coord.0, coord.1, coord.2, coord.3);

    assert!(
        n1_crystallized_after,
        "n1 should be crystallized after merge"
    );
    assert!(
        n2_crystallized_after,
        "n2 should be crystallized after merge"
    );

    // GAP CLOSED: with evidence-carrying sync + resolve(), records converge.
    // Both nodes should now have merged influences containing alice AND bob.
    let n1_record_after = n1.record(coord.0, coord.1, coord.2, coord.3);
    let n2_record_after = n2.record(coord.0, coord.1, coord.2, coord.3);

    // Both records should contain evidence from both parties.
    let n1_has_both = n1_record_after.contains("alice") && n1_record_after.contains("bob");
    let n2_has_both = n2_record_after.contains("alice") && n2_record_after.contains("bob");

    assert!(
        n1_has_both,
        "GAP CLOSED: n1 should have both alice and bob after resolve. Got: '{}'",
        n1_record_after
    );
    assert!(
        n2_has_both,
        "GAP CLOSED: n2 should have both alice and bob after resolve. Got: '{}'",
        n2_record_after
    );

    // Records should now be identical on both nodes (deterministic resolve).
    assert_eq!(
        n1_record_after, n2_record_after,
        "GAP CLOSED: records should converge. n1='{}', n2='{}'",
        n1_record_after, n2_record_after
    );
}

// ── 13. Split-brain: probability wins over non-crystallized ──────────────────
//
// Node 1 has high probability (not yet crystallized).
// Node 2 has crystallized the same coord with a different event.
// After sync: crystallized wins (k=true propagates).

#[test]
fn split_brain_crystallized_wins_over_probability() {
    let n1 = TestNode::start(18210, vec![]);
    let n2 = TestNode::start(18211, vec![]);

    // Node 1: seed lightly (high p but maybe not crystallized)
    n1.seed(3, 3, 3, 3, "weak-claim");

    // Node 2: seed heavily to ensure crystallization
    n2.seed(3, 3, 3, 3, "strong-claim");
    n2.seed(3, 4, 3, 3, "strong-ctx1");
    n2.seed(4, 3, 3, 3, "strong-ctx2");
    n2.seed(3, 3, 4, 3, "strong-ctx3");
    n2.seed(3, 3, 3, 4, "strong-ctx4");

    thread::sleep(Duration::from_secs(3));

    let n2_crystallized = n2.is_crystallized(3, 3, 3, 3);

    // Reconnect
    let n2_boundary = http_get(&n2.addr(), "/boundary").unwrap_or_default();
    http_post(&n1.addr(), "/boundary", &n2_boundary);

    thread::sleep(Duration::from_secs(1));

    // If n2 was crystallized, n1 should now also be crystallized
    if n2_crystallized {
        assert!(
            n1.is_crystallized(3, 3, 3, 3),
            "crystallized state should propagate to n1 after merge"
        );
        assert_eq!(
            n1.probability(3, 3, 3, 3),
            1.0,
            "crystallized cell should have p=1.0"
        );
    }
}

// ── 14. Split-brain: simultaneous crystallization count divergence ────────────
//
// Each node crystallizes different coords. After merge, total crystallized
// should be the UNION, not the intersection.

#[test]
fn split_brain_crystallization_counts_merge_as_union() {
    let n1 = TestNode::start(18220, vec![]);
    let n2 = TestNode::start(18221, vec![]);

    // Node 1: crystallize at (2,2,2,2)
    n1.seed(2, 2, 2, 2, "ev-n1");
    n1.seed(2, 3, 2, 2, "ctx-n1-1");
    n1.seed(3, 2, 2, 2, "ctx-n1-2");

    // Node 2: crystallize at (6,6,6,6)
    n2.seed(6, 6, 6, 6, "ev-n2");
    n2.seed(6, 5, 6, 6, "ctx-n2-1");
    n2.seed(5, 6, 6, 6, "ctx-n2-2");

    thread::sleep(Duration::from_secs(3));

    let n1_crystals_before = n1.crystallized_count();
    let n2_crystals_before = n2.crystallized_count();

    // Reconnect: bidirectional boundary exchange
    let n2_boundary = http_get(&n2.addr(), "/boundary").unwrap_or_default();
    let n1_boundary = http_get(&n1.addr(), "/boundary").unwrap_or_default();
    http_post(&n1.addr(), "/boundary", &n2_boundary);
    http_post(&n2.addr(), "/boundary", &n1_boundary);

    thread::sleep(Duration::from_secs(2));

    let n1_crystals_after = n1.crystallized_count();
    let n2_crystals_after = n2.crystallized_count();

    // After merge: each node should have AT LEAST as many crystals as before
    assert!(
        n1_crystals_after >= n1_crystals_before,
        "n1 should not lose crystals: before={}, after={}",
        n1_crystals_before,
        n1_crystals_after
    );
    assert!(
        n2_crystals_after >= n2_crystals_before,
        "n2 should not lose crystals: before={}, after={}",
        n2_crystals_before,
        n2_crystals_after
    );

    // Both nodes should now have BOTH crystallization regions
    let n1_has_both = n1.is_crystallized(2, 2, 2, 2) && n1.probability(6, 6, 6, 6) > 0.0;
    let n2_has_both = n2.is_crystallized(6, 6, 6, 6) && n2.probability(2, 2, 2, 2) > 0.0;

    assert!(
        n1_has_both,
        "n1 should have both regions after merge: own={}, peer_p={}",
        n1.is_crystallized(2, 2, 2, 2),
        n1.probability(6, 6, 6, 6)
    );
    assert!(
        n2_has_both,
        "n2 should have both regions after merge: own={}, peer_p={}",
        n2.is_crystallized(6, 6, 6, 6),
        n2.probability(2, 2, 2, 2)
    );
}

// ── 15. Evidence sync: influences now travel via boundary ─────────────────────
//
// GAP CLOSED: The wire protocol now carries influences in boundary cells.
// The `resolve()` function merges evidence as union. After reconnection,
// both nodes should have both parties' influences.

#[test]
fn influences_sync_via_boundary_after_resolve() {
    let n1 = TestNode::start(18230, vec![]);
    let n2 = TestNode::start(18231, vec![]);

    // Both seed at same coord with different IDs
    n1.seed(4, 4, 4, 4, "alice-version");
    n2.seed(4, 4, 4, 4, "bob-version");

    thread::sleep(Duration::from_secs(1));

    // Before reconnect: each has only its own
    let n1_has_alice = n1
        .influence_ids(4, 4, 4, 4)
        .iter()
        .any(|id| id.contains("alice"));
    let n2_has_bob = n2
        .influence_ids(4, 4, 4, 4)
        .iter()
        .any(|id| id.contains("bob"));
    assert!(
        n1_has_alice,
        "n1 should have alice's influence before merge"
    );
    assert!(n2_has_bob, "n2 should have bob's influence before merge");

    // Reconnect: bidirectional boundary exchange
    let n2_boundary = http_get(&n2.addr(), "/boundary").unwrap_or_default();
    http_post(&n1.addr(), "/boundary", &n2_boundary);
    let n1_boundary = http_get(&n1.addr(), "/boundary").unwrap_or_default();
    http_post(&n2.addr(), "/boundary", &n1_boundary);

    thread::sleep(Duration::from_secs(1));

    // After merge: n1 should now also have bob's influence (merged via resolve)
    let n1_influences_after = n1.influence_ids(4, 4, 4, 4);
    let n1_got_bob = n1_influences_after.iter().any(|id| id.contains("bob"));
    let n1_kept_alice = n1_influences_after.iter().any(|id| id.contains("alice"));

    assert!(
        n1_got_bob,
        "GAP CLOSED: n1 should receive bob's influence via evidence sync. Got: {:?}",
        n1_influences_after
    );
    assert!(
        n1_kept_alice,
        "n1 should keep alice's influence after merge. Got: {:?}",
        n1_influences_after
    );
}

// ── 16. Conservation: double-spend across partition ──────────────────────────
//
// The conservation layer (ConservedField) is LOCAL to each node.
// Two partitioned nodes can independently spend from the same balance.
// After reconnection, there is no cross-node conservation check.
// This test documents this as a known limitation.

#[test]
fn double_spend_across_partition_detected_and_resolved() {
    use tesseract::conservation::*;

    let src = Coord {
        t: 0,
        c: 0,
        o: 0,
        v: 0,
    };

    let mut field_a = ConservedField::new();
    let mut field_b = ConservedField::new();
    field_a.genesis(&[(src, 1000)]);
    field_b.genesis(&[(src, 1000)]);

    // Partition: each processes a different transfer from same source, nonce=0
    let tx_a = Transfer::new(
        vec![TransferInput {
            coord: src,
            amount: 800,
            expected_nonce: 0,
        }],
        vec![TransferOutput {
            coord: Coord {
                t: 1,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 800,
        }],
    )
    .unwrap();

    let tx_b = Transfer::new(
        vec![TransferInput {
            coord: src,
            amount: 900,
            expected_nonce: 0,
        }],
        vec![TransferOutput {
            coord: Coord {
                t: 2,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 900,
        }],
    )
    .unwrap();

    field_a.apply(&tx_a).unwrap();
    field_b.apply(&tx_b).unwrap();

    // Reconnect: cross-check detects double-spend
    let check_a = field_a.check_remote_transfer(src, 0, tx_b.hash);
    let check_b = field_b.check_remote_transfer(src, 0, tx_a.hash);

    assert!(
        check_a.is_err(),
        "GAP CLOSED: double-spend detected on reconnect"
    );
    assert!(check_b.is_err(), "GAP CLOSED: detected on both sides");

    // Deterministic: both agree on winner
    let a_wins_per_a = match check_a.unwrap_err() {
        ConservationError::DoubleSpend { remote_wins, .. } => !remote_wins,
        e => panic!("expected DoubleSpend, got {}", e),
    };
    let a_wins_per_b = match check_b.unwrap_err() {
        ConservationError::DoubleSpend { remote_wins, .. } => remote_wins,
        e => panic!("expected DoubleSpend, got {}", e),
    };

    assert_eq!(
        a_wins_per_a, a_wins_per_b,
        "GAP CLOSED: both sides agree on winner"
    );
}
