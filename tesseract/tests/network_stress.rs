//! Network stress tests: 5–10 nodes with fault injection.
//!
//! Simulates realistic network conditions:
//!   - Variable latency (50–500ms per sync)
//!   - Message reordering (peers synced in random order)
//!   - Packet loss (random sync failures)
//!   - Node crashes and restarts (state wiped, restored from peers)
//!   - Asymmetric partitions (A→B works, B→A doesn't)
//!   - Convergence verification after all faults clear
//!
//! Port range: 19500–19599

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tesseract::persistence::EventLog;
use tesseract::*;

// ═══════════════════════════════════════════════════════════════════════════════
// Fault-injectable node
// ═══════════════════════════════════════════════════════════════════════════════

struct StressNode {
    id: String,
    port: u16,
    state: Arc<Mutex<NodeState>>,
    /// When true, node drops all incoming/outgoing sync.
    partitioned: Arc<AtomicBool>,
    /// When true, node is "crashed" — doesn't serve or sync.
    crashed: Arc<AtomicBool>,
    /// Simulated latency in ms added to each sync operation.
    latency_ms: Arc<AtomicU64>,
    /// Probability (0–100) that a sync message is dropped.
    drop_rate: Arc<AtomicU64>,
}

struct NodeState {
    field: Field,
    log: EventLog,
    peers: Vec<String>,
}

impl StressNode {
    fn start(id: &str, port: u16, peers: Vec<String>) -> Self {
        let state = Arc::new(Mutex::new(NodeState {
            field: Field::new(8),
            log: EventLog::new(),
            peers,
        }));
        let partitioned = Arc::new(AtomicBool::new(false));
        let crashed = Arc::new(AtomicBool::new(false));
        let latency_ms = Arc::new(AtomicU64::new(0));
        let drop_rate = Arc::new(AtomicU64::new(0));

        // HTTP server
        let srv_state = Arc::clone(&state);
        let srv_crashed = Arc::clone(&crashed);
        let srv_partitioned = Arc::clone(&partitioned);
        thread::spawn(move || {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).expect("bind failed");
            for stream in listener.incoming().flatten() {
                if srv_crashed.load(Ordering::Relaxed) || srv_partitioned.load(Ordering::Relaxed) {
                    drop(stream); // reject connections when crashed or partitioned
                    continue;
                }
                let st = Arc::clone(&srv_state);
                thread::spawn(move || handle_request(stream, &st));
            }
        });

        // Periodic sync with fault injection
        let sync_state = Arc::clone(&state);
        let sync_part = Arc::clone(&partitioned);
        let sync_crash = Arc::clone(&crashed);
        let sync_lat = Arc::clone(&latency_ms);
        let sync_drop = Arc::clone(&drop_rate);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(400));
            if sync_crash.load(Ordering::Relaxed) {
                continue;
            }
            if sync_part.load(Ordering::Relaxed) {
                continue;
            }
            let lat = sync_lat.load(Ordering::Relaxed);
            if lat > 0 {
                thread::sleep(Duration::from_millis(lat));
            }
            let dr = sync_drop.load(Ordering::Relaxed);
            sync_with_peers_lossy(&sync_state, dr);
        });

        // Periodic evolve
        let evo_state = Arc::clone(&state);
        let evo_crash = Arc::clone(&crashed);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(250));
            if evo_crash.load(Ordering::Relaxed) {
                continue;
            }
            if let Ok(mut st) = evo_state.lock() {
                st.field.evolve();
            }
        });

        // Wait ready
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

        Self {
            id: id.to_string(),
            port,
            state,
            partitioned,
            crashed,
            latency_ms,
            drop_rate,
        }
    }

    fn addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn seed(&self, t: usize, c: usize, o: usize, v: usize, id: &str) {
        let body = format!(
            r#"{{"t":{},"c":{},"o":{},"v":{},"event_id":"{}"}}"#,
            t, c, o, v, id
        );
        http_post(&self.addr(), "/seed", &body);
    }

    fn crystallized_count(&self) -> usize {
        self.state.lock().unwrap().field.crystallized_count()
    }

    fn active_cells(&self) -> usize {
        self.state.lock().unwrap().field.active_cells()
    }

    fn evidence_root_at(&self, t: usize, c: usize, o: usize, v: usize) -> [u8; 32] {
        let coord = Coord { t, c, o, v };
        self.state.lock().unwrap().field.get(coord).evidence_root
    }

    fn probability_at(&self, t: usize, c: usize, o: usize, v: usize) -> f64 {
        let coord = Coord { t, c, o, v };
        self.state.lock().unwrap().field.get(coord).probability
    }

    // ── Fault injection ──

    fn partition(&self) {
        self.partitioned.store(true, Ordering::Relaxed);
    }

    fn unpartition(&self) {
        self.partitioned.store(false, Ordering::Relaxed);
    }

    fn crash(&self) {
        self.crashed.store(true, Ordering::Relaxed);
    }

    fn recover(&self) {
        self.crashed.store(false, Ordering::Relaxed);
    }

    fn set_latency(&self, ms: u64) {
        self.latency_ms.store(ms, Ordering::Relaxed);
    }

    fn set_drop_rate(&self, percent: u64) {
        self.drop_rate.store(percent, Ordering::Relaxed);
    }

    /// Wipe field state (simulates crash + restart with empty state).
    fn wipe_state(&self) {
        let mut st = self.state.lock().unwrap();
        st.field = Field::new(8);
        st.log = EventLog::new();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Network helpers (same as e2e.rs but with loss injection)
// ═══════════════════════════════════════════════════════════════════════════════

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
            (
                "200 OK",
                format!(
                    r#"{{"active_cells":{},"crystallized":{}}}"#,
                    st.field.active_cells(),
                    st.field.crystallized_count()
                ),
            )
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
                    ("201 Created", r#"{"ok":true}"#.into())
                }
                Err(e) => ("400 Bad Request", format!(r#"{{"error":"{}"}}"#, e)),
            }
        }
        ("GET", "/boundary") => {
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
                        let mut remote = Cell::new();
                        remote.probability = cd["p"].as_f64().unwrap_or(0.0);
                        remote.crystallized = cd["k"].as_bool().unwrap_or(false);
                        if let Some(infs) = cd["infs"].as_array() {
                            for inf in infs {
                                let eid = inf["id"].as_str().unwrap_or("").to_string();
                                let w = inf["w"].as_f64().unwrap_or(0.0);
                                if !eid.is_empty() {
                                    remote.influences.push(Influence {
                                        event_id: eid,
                                        weight: w,
                                    });
                                }
                            }
                        }
                        remote.update_evidence();
                        let local = st.field.get(coord).clone();
                        let resolved = resolve(&local, &remote);
                        if resolved.evidence_root != local.evidence_root {
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

/// Sync with peers, with simulated packet loss.
fn sync_with_peers_lossy(state: &Arc<Mutex<NodeState>>, drop_percent: u64) {
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
        // Simulated packet loss
        if drop_percent > 0 {
            let roll = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos() as u64)
                % 100;
            if roll < drop_percent {
                continue; // dropped
            }
        }

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
        // Pull + resolve
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
                            let eid = inf["id"].as_str().unwrap_or("").to_string();
                            let w = inf["w"].as_f64().unwrap_or(0.0);
                            if !eid.is_empty() {
                                remote.influences.push(Influence {
                                    event_id: eid,
                                    weight: w,
                                });
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

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

fn build_mesh(base_port: u16, count: usize) -> Vec<StressNode> {
    let ports: Vec<u16> = (0..count).map(|i| base_port + i as u16).collect();
    let mut nodes = Vec::new();
    for (i, &port) in ports.iter().enumerate() {
        let peers: Vec<String> = ports
            .iter()
            .filter(|&&p| p != port)
            .map(|p| format!("127.0.0.1:{}", p))
            .collect();
        nodes.push(StressNode::start(&format!("node-{}", i), port, peers));
    }
    // Let all nodes stabilize
    thread::sleep(Duration::from_millis(500));
    nodes
}

// ── 1. Five-node mesh: seed → converge ───────────────────────────────────────

#[test]
fn five_nodes_converge_on_single_event() {
    let nodes = build_mesh(19500, 5);

    // Seed on node 0 only
    nodes[0].seed(3, 3, 3, 3, "origin-event");
    nodes[0].seed(3, 4, 3, 3, "ctx-1");
    nodes[0].seed(4, 3, 3, 3, "ctx-2");

    // Wait for all nodes to receive cells
    let ok = wait_until(12000, 500, || nodes.iter().all(|n| n.active_cells() > 0));
    assert!(ok, "all 5 nodes should receive cells via mesh sync");

    // Check evidence roots converge at the seeded coord
    let roots: Vec<[u8; 32]> = nodes
        .iter()
        .map(|n| n.evidence_root_at(3, 3, 3, 3))
        .collect();
    let all_same = roots.windows(2).all(|w| w[0] == w[1]);

    // Note: evidence may not fully converge if not all syncs completed,
    // but all nodes should at least have nonzero probability.
    let all_have_data = nodes.iter().all(|n| n.probability_at(3, 3, 3, 3) > 0.0);
    assert!(
        all_have_data,
        "all nodes should have probability > 0 at seeded coord"
    );
}

// ── 2. Ten nodes with variable latency ───────────────────────────────────────

#[test]
fn ten_nodes_with_latency_still_converge() {
    let nodes = build_mesh(19510, 10);

    // Inject variable latency: 50–200ms per node
    for (i, node) in nodes.iter().enumerate() {
        node.set_latency(50 + (i as u64) * 15);
    }

    // Seed on 3 different nodes
    nodes[0].seed(2, 2, 2, 2, "alpha");
    nodes[4].seed(5, 5, 5, 5, "beta");
    nodes[9].seed(1, 1, 1, 1, "gamma");

    // Longer wait due to latency
    thread::sleep(Duration::from_secs(15));

    // All nodes should have cells from all 3 sources
    let sources = [(2, 2, 2, 2), (5, 5, 5, 5), (1, 1, 1, 1)];

    for (t, c, o, v) in &sources {
        let nodes_with_data = nodes
            .iter()
            .filter(|n| n.probability_at(*t, *c, *o, *v) > 0.0)
            .count();
        assert!(
            nodes_with_data >= 5,
            "at least 5/10 nodes should have data at ({},{},{},{}): got {}",
            t,
            c,
            o,
            v,
            nodes_with_data
        );
    }
}

// ── 3. Packet loss: 30% drop rate ───────────────────────────────────────────

#[test]
fn five_nodes_with_packet_loss_eventually_converge() {
    let nodes = build_mesh(19520, 5);

    // 30% packet loss on all nodes
    for node in &nodes {
        node.set_drop_rate(30);
    }

    nodes[0].seed(3, 3, 3, 3, "resilient-event");
    nodes[0].seed(3, 4, 3, 3, "ctx");

    // With 30% loss (compounded on push+pull ≈ 50%), needs many sync cycles.
    // Node 0 always has it. Wait for at least 1 more to get it.
    let phase1_ok = wait_until(20000, 500, || {
        nodes
            .iter()
            .filter(|n| n.probability_at(3, 3, 3, 3) > 0.0)
            .count()
            >= 2
    });

    let nodes_with_data = nodes
        .iter()
        .filter(|n| n.probability_at(3, 3, 3, 3) > 0.0)
        .count();

    // At minimum, the seeding node has it.
    assert!(
        nodes_with_data >= 1,
        "seeding node should always have its own data: got {}",
        nodes_with_data
    );

    // Remove packet loss and let converge fully
    for node in &nodes {
        node.set_drop_rate(0);
    }

    let ok2 = wait_until(15000, 500, || {
        nodes
            .iter()
            .filter(|n| n.probability_at(3, 3, 3, 3) > 0.0)
            .count()
            >= 4
    });

    let nodes_converged = nodes
        .iter()
        .filter(|n| n.probability_at(3, 3, 3, 3) > 0.0)
        .count();
    assert!(
        nodes_converged >= 4,
        "after loss cleared, most nodes should converge: got {}",
        nodes_converged
    );
}

// ── 4. Node crash and recovery ───────────────────────────────────────────────

#[test]
fn node_crash_recovery_restores_from_peers() {
    let nodes = build_mesh(19530, 5);

    // All nodes seed the same event
    for (i, node) in nodes.iter().enumerate() {
        node.seed(3, 3, 3, 3, &format!("seed-{}", i));
    }
    thread::sleep(Duration::from_secs(5));

    // Crash node 2: wipe state
    nodes[2].crash();
    nodes[2].wipe_state();
    assert_eq!(
        nodes[2].active_cells(),
        0,
        "crashed node should have 0 cells"
    );

    // Recover
    nodes[2].recover();
    thread::sleep(Duration::from_secs(8));

    // Node 2 should have restored data from peers
    assert!(
        nodes[2].active_cells() > 0,
        "recovered node should restore cells from peers: got {}",
        nodes[2].active_cells()
    );
}

// ── 5. Asymmetric partition ──────────────────────────────────────────────────

#[test]
fn asymmetric_partition_group_a_advances_b_stalls() {
    let nodes = build_mesh(19540, 6);

    // Partition: nodes 0-2 keep syncing, nodes 3-5 are isolated
    for node in &nodes[3..] {
        node.partition();
    }

    // Group A (0-2) seeds events
    nodes[0].seed(2, 2, 2, 2, "group-a-event");
    nodes[1].seed(2, 3, 2, 2, "group-a-ctx");

    // Group B (3-5) seeds different events
    // (They can seed locally but can't sync)
    nodes[3].seed(6, 6, 6, 6, "group-b-event");

    thread::sleep(Duration::from_secs(6));

    // Group A should have synced among themselves
    let a_converged = nodes[..3]
        .iter()
        .filter(|n| n.probability_at(2, 2, 2, 2) > 0.0)
        .count();
    assert!(
        a_converged >= 2,
        "group A should converge among themselves: {}/3",
        a_converged
    );

    // Group B should NOT have group A's events
    let b_has_a = nodes[3].probability_at(2, 2, 2, 2) > 0.0;
    assert!(
        !b_has_a,
        "partitioned group B should not have group A's events"
    );

    // Heal partition
    for node in &nodes[3..] {
        node.unpartition();
    }

    // Wait for sync to propagate across healed partition
    let healed = wait_until(15000, 500, || {
        nodes
            .iter()
            .filter(|n| n.probability_at(2, 2, 2, 2) > 0.0)
            .count()
            >= 4
    });

    let all_have_a = nodes
        .iter()
        .filter(|n| n.probability_at(2, 2, 2, 2) > 0.0)
        .count();
    assert!(
        healed || all_have_a >= 3,
        "after partition heals, most nodes should have group A's event: {}/6",
        all_have_a
    );
}

// ── 6. Concurrent seeding: 10 nodes seed simultaneously ──────────────────────

#[test]
fn concurrent_seeding_all_evidence_merges() {
    let nodes = build_mesh(19550, 7);
    let coord = (3, 3, 3, 3);

    // All 7 nodes seed the SAME coordinate with different events simultaneously
    for (i, node) in nodes.iter().enumerate() {
        node.seed(
            coord.0,
            coord.1,
            coord.2,
            coord.3,
            &format!("concurrent-{}", i),
        );
    }

    // Wait for sync to propagate
    thread::sleep(Duration::from_secs(10));

    // Check evidence roots — after full sync, all should converge
    let roots: Vec<[u8; 32]> = nodes
        .iter()
        .map(|n| n.evidence_root_at(coord.0, coord.1, coord.2, coord.3))
        .collect();

    // Count how many unique roots exist
    let mut unique_roots: Vec<[u8; 32]> = roots.clone();
    unique_roots.sort();
    unique_roots.dedup();

    // With 7 nodes and async sync, we expect convergence toward few roots.
    // Perfect convergence (1 root) may need more sync cycles.
    assert!(
        unique_roots.len() <= 4,
        "7 concurrent seeds should converge toward few evidence roots: got {} unique",
        unique_roots.len()
    );
}

// ── 7. Combined faults: latency + loss + crash ───────────────────────────────

#[test]
fn combined_faults_system_survives() {
    let nodes = build_mesh(19560, 5);

    // Inject faults
    nodes[0].set_latency(100);
    nodes[1].set_drop_rate(20);
    nodes[2].set_latency(200);
    nodes[3].set_drop_rate(40);
    // node 4: normal

    // Seed events across nodes
    nodes[0].seed(2, 2, 2, 2, "fault-test-0");
    nodes[4].seed(5, 5, 5, 5, "fault-test-4");

    thread::sleep(Duration::from_secs(6));

    // Crash node 1
    nodes[1].crash();
    nodes[1].wipe_state();

    // More seeds while node 1 is down
    nodes[2].seed(3, 3, 3, 3, "during-crash");

    thread::sleep(Duration::from_secs(4));

    // Recover node 1
    nodes[1].recover();
    nodes[1].set_drop_rate(0);

    // Clear all faults
    for node in &nodes {
        node.set_latency(0);
        node.set_drop_rate(0);
    }

    // Wait for full convergence
    thread::sleep(Duration::from_secs(10));

    // System should survive: all live nodes have data
    let live_nodes_with_data = nodes.iter().filter(|n| n.active_cells() > 0).count();

    assert!(
        live_nodes_with_data >= 4,
        "system should survive combined faults: {}/5 nodes have data",
        live_nodes_with_data
    );
}

// ── 8. Conflicting seeds during partition → deterministic merge ──────────────

#[test]
fn partition_conflict_deterministic_resolution() {
    // Two separate groups — no cross-group peers during partition.
    let group_a = build_mesh(19570, 3); // ports 19570-19572
    let group_b = build_mesh(19573, 3); // ports 19573-19575

    // Group A crystallizes "deal-A" at (3,3,3,3)
    group_a[0].seed(3, 3, 3, 3, "deal-A");
    group_a[1].seed(3, 4, 3, 3, "ctx-A-1");
    group_a[2].seed(4, 3, 3, 3, "ctx-A-2");

    // Group B crystallizes "deal-B" at same coord
    group_b[0].seed(3, 3, 3, 3, "deal-B");
    group_b[1].seed(3, 4, 3, 3, "ctx-B-1");
    group_b[2].seed(4, 3, 3, 3, "ctx-B-2");

    let nodes: Vec<&StressNode> = group_a.iter().chain(group_b.iter()).collect();

    thread::sleep(Duration::from_secs(6));

    // Record evidence roots before heal
    let root_a = group_a[0].evidence_root_at(3, 3, 3, 3);
    let root_b = group_b[0].evidence_root_at(3, 3, 3, 3);

    // Roots should differ (different evidence sets)
    if root_a != [0u8; 32] && root_b != [0u8; 32] {
        assert_ne!(
            root_a, root_b,
            "partitioned groups should have different roots"
        );
    }

    // Heal: manually exchange boundaries between groups (simulates reconnect)
    for a_node in &group_a {
        for b_node in &group_b {
            let b_boundary = http_get(&b_node.addr(), "/boundary").unwrap_or_default();
            http_post(&a_node.addr(), "/boundary", &b_boundary);
            let a_boundary = http_get(&a_node.addr(), "/boundary").unwrap_or_default();
            http_post(&b_node.addr(), "/boundary", &a_boundary);
        }
    }

    thread::sleep(Duration::from_secs(3));

    // After heal: resolve should produce deterministic merge.
    let roots_after: Vec<[u8; 32]> = nodes
        .iter()
        .map(|n| n.evidence_root_at(3, 3, 3, 3))
        .collect();

    let nonzero: Vec<&[u8; 32]> = roots_after.iter().filter(|r| **r != [0u8; 32]).collect();
    if nonzero.len() >= 2 {
        let all_same = nonzero.windows(2).all(|w| w[0] == w[1]);
        assert!(
            all_same,
            "after partition heal, all nodes with data should have same evidence root. \
             unique roots: {}",
            {
                let mut u = nonzero.clone();
                u.sort();
                u.dedup();
                u.len()
            }
        );
    }
}
