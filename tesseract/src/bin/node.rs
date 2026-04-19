//! Tesseract node: raw TCP HTTP server + periodic peer sync.
//! Zero HTTP dependencies — stdlib only.

use std::env;
use std::io::{Read, Write, BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tesseract::*;
use tesseract::persistence::EventLog;

struct AppState {
    node_id: String,
    field: Field,
    log: EventLog,
    region_id: usize,
    peers: Vec<String>,
}

fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "7700".into());
    let node_id = env::var("NODE_ID").unwrap_or_else(|_| "node-0".into());
    let field_size: usize = env::var("FIELD_SIZE").unwrap_or_else(|_| "8".into()).parse().unwrap_or(8);
    let peers: Vec<String> = env::var("PEERS").unwrap_or_default()
        .split(',').filter(|s| !s.is_empty()).map(|s| s.trim().to_string()).collect();
    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| "./data".into());
    let region_id: usize = env::var("REGION_ID").unwrap_or_else(|_| "0".into()).parse().unwrap_or(0);
    let genesis_alloc: f64 = env::var("GENESIS_ALLOC").unwrap_or_else(|_| "50000".into()).parse().unwrap_or(50000.0);

    let _ = std::fs::create_dir_all(&data_dir);
    let log_path = format!("{}/{}.log", data_dir, node_id);
    let log = EventLog::with_file(&log_path);

    let mut field = Field::new(field_size);
    field.set_capacity(region_id, genesis_alloc);
    log.replay(&mut field);
    evolve_to_equilibrium(&mut field, 5);

    let state = Arc::new(Mutex::new(AppState {
        node_id: node_id.clone(), field, log, region_id, peers: peers.clone(),
    }));

    eprintln!("╔══════════════════════════════════════╗");
    eprintln!("║       TESSERACT NODE                 ║");
    eprintln!("╚══════════════════════════════════════╝");
    eprintln!("  Node:   {}", node_id);
    eprintln!("  Region: {}", region_id);
    eprintln!("  Field:  {}⁴", field_size);
    eprintln!("  Port:   {}", port);
    eprintln!("  Peers:  {}", if peers.is_empty() { "none".into() } else { peers.join(", ") });

    // Periodic sync
    let sync_st = Arc::clone(&state);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(5));
        sync_with_peers(&sync_st);
    });

    // Periodic evolve
    let evo_st = Arc::clone(&state);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2));
        if let Ok(mut st) = evo_st.lock() { st.field.evolve(); }
    });

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("bind failed");
    eprintln!("  Listening on 0.0.0.0:{}", port);

    for stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        thread::spawn(move || handle_request(stream, &state));
    }
}

fn handle_request(mut stream: TcpStream, state: &Arc<Mutex<AppState>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(&stream);

    // Parse request line
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() { return; }
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 { return; }
    let method = parts[0];
    let path = parts[1];

    // Parse headers to get content-length
    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() { break; }
        if header.trim().is_empty() { break; }
        if header.to_lowercase().starts_with("content-length:") {
            content_length = header.split(':').nth(1)
                .and_then(|v| v.trim().parse().ok()).unwrap_or(0);
        }
    }

    // Read body
    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        let _ = reader.read_exact(&mut buf);
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    let (status, response_body) = route(method, path, &body, state);
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, response_body.len(), response_body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn route(method: &str, path: &str, body: &str, state: &Arc<Mutex<AppState>>) -> (&'static str, String) {
    match (method, path) {
        ("GET", "/status") => {
            let st = state.lock().unwrap();
            let json = format!(
                r#"{{"node_id":"{}","region":{},"field_size":{},"active_cells":{},"crystallized":{},"curvature":{:.2},"peers":{},"events":{}}}"#,
                st.node_id, st.region_id, st.field.size, st.field.active_cells(),
                st.field.crystallized_count(), st.field.capacity(st.region_id).unwrap_or(0.0),
                st.peers.len(), st.log.len()
            );
            ("200 OK", json)
        }

        ("POST", "/seed") => {
            // Parse: {"t":1,"c":2,"o":3,"v":4,"event_id":"tx-001"}
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
                        r#"{{"coord":"{}","probability":{:.4},"crystallized":{},"support":{}}}"#,
                        coord, cell.probability, cell.crystallized, st.field.orthogonal_support(coord)
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
                    let coord = Coord { t, c, o, v: vv };
                    st.field.destroy(coord);
                    ("200 OK", r#"{"destroyed":true}"#.into())
                }
                Err(e) => ("400 Bad Request", format!(r#"{{"error":"{}"}}"#, e)),
            }
        }

        ("GET", p) if p.starts_with("/cell/") => {
            let nums: Vec<usize> = p.trim_start_matches("/cell/")
                .split('/').filter_map(|s| s.parse().ok()).collect();
            if nums.len() == 4 {
                let st = state.lock().unwrap();
                let coord = Coord { t: nums[0], c: nums[1], o: nums[2], v: nums[3] };
                let cell = st.field.get(coord);
                let json = format!(
                    r#"{{"coord":"{}","probability":{:.4},"crystallized":{},"support":{},"record":"{}"}}"#,
                    coord, cell.probability, cell.crystallized,
                    st.field.orthogonal_support(coord), cell.record().replace('"', "'")
                );
                ("200 OK", json)
            } else {
                ("400 Bad Request", r#"{"error":"use /cell/t/c/o/v"}"#.into())
            }
        }

        ("GET", "/boundary") => {
            let st = state.lock().unwrap();
            let cells: Vec<String> = st.field.active_entries()
                .filter(|(_, cell)| cell.crystallized || cell.probability > 0.1)
                .map(|(coord, cell)| format!(
                    r#"{{"t":{},"c":{},"o":{},"v":{},"p":{:.4},"k":{}}}"#,
                    coord.t, coord.c, coord.o, coord.v, cell.probability, cell.crystallized
                ))
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

                        let local = st.field.get_mut(coord);
                        if p > local.probability { local.probability = p; merged += 1; }
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

fn sync_with_peers(state: &Arc<Mutex<AppState>>) {
    let (peers, boundary_json) = {
        let st = state.lock().unwrap();
        let cells: Vec<String> = st.field.active_entries()
            .filter(|(_, cell)| cell.crystallized || cell.probability > 0.1)
            .map(|(coord, cell)| format!(
                r#"{{"t":{},"c":{},"o":{},"v":{},"p":{:.4},"k":{}}}"#,
                coord.t, coord.c, coord.o, coord.v, cell.probability, cell.crystallized
            ))
            .collect();
        (st.peers.clone(), format!("[{}]", cells.join(",")))
    };

    for peer in &peers {
        let addr = peer.trim_start_matches("http://");
        let socket_addr = match addr.parse() {
            Ok(a) => a,
            Err(_) => continue,
        };

        // 1. Push our boundary to peer
        if let Ok(mut stream) = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)) {
            let req = format!(
                "POST /boundary HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                addr, boundary_json.len(), boundary_json
            );
            let _ = stream.write_all(req.as_bytes());
        }

        // 2. Pull peer's boundary into our field
        if let Ok(mut stream) = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)) {
            let req = format!(
                "GET /boundary HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                addr
            );
            if stream.write_all(req.as_bytes()).is_ok() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                let mut response = Vec::new();
                let _ = stream.read_to_end(&mut response);
                let text = String::from_utf8_lossy(&response);
                // Skip HTTP headers — body starts after \r\n\r\n
                if let Some(body_start) = text.find("\r\n\r\n") {
                    let body = &text[body_start + 4..];
                    if let Ok(cells) = serde_json::from_str::<Vec<serde_json::Value>>(body) {
                        let mut st = state.lock().unwrap();
                        for cd in &cells {
                            let coord = Coord {
                                t: cd["t"].as_u64().unwrap_or(0) as usize,
                                c: cd["c"].as_u64().unwrap_or(0) as usize,
                                o: cd["o"].as_u64().unwrap_or(0) as usize,
                                v: cd["v"].as_u64().unwrap_or(0) as usize,
                            };
                            let p = cd["p"].as_f64().unwrap_or(0.0);
                            let k = cd["k"].as_bool().unwrap_or(false);

                            let local = st.field.get_mut(coord);
                            if p > local.probability { local.probability = p; }
                            if k && !local.crystallized {
                                local.crystallized = true;
                                local.probability = 1.0;
                            }
                        }
                    }
                }
            }
        }
    }
}
