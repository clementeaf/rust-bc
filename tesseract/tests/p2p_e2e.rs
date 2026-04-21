//! P2P end-to-end test: two nodes exchange events over real TCP.

use tesseract::p2p;
use tesseract::Coord;
use std::time::Duration;

#[tokio::test]
async fn two_nodes_gossip_event() {
    // Start node A on a random port
    let (handle_a, mut state_a) = p2p::start("node-a", "127.0.0.1:0", &[])
        .await.expect("node A failed to start");

    // We need the actual bound port — re-bind workaround:
    // Start node A on a fixed port for simplicity
    drop(handle_a);
    drop(state_a);

    let (handle_a, mut state_a) = p2p::start("node-a", "127.0.0.1:19100", &[])
        .await.expect("node A failed to start");

    // Give listener time to bind
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Start node B connecting to node A
    let (handle_b, mut state_b) = p2p::start(
        "node-b",
        "127.0.0.1:19101",
        &["127.0.0.1:19100".to_string()],
    ).await.expect("node B failed to start");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Node B gossips an event
    let coord = Coord { t: 5, c: 5, o: 5, v: 5 };
    p2p::gossip_seed(&handle_b, coord, "tx-from-b").await;

    // Node A should receive the event
    let received = tokio::time::timeout(
        Duration::from_secs(2),
        state_a.event_rx.recv(),
    ).await;

    assert!(received.is_ok(), "node A should receive event within 2s");
    let (recv_coord, recv_id) = received.unwrap().expect("channel closed");
    assert_eq!(recv_coord, coord);
    assert_eq!(recv_id, "tx-from-b");
}

#[tokio::test]
async fn boundary_sync_between_nodes() {
    let (handle_a, mut state_a) = p2p::start("node-a", "127.0.0.1:19200", &[])
        .await.expect("node A failed to start");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let (handle_b, _state_b) = p2p::start(
        "node-b",
        "127.0.0.1:19201",
        &["127.0.0.1:19200".to_string()],
    ).await.expect("node B failed to start");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Node B sends boundary cells
    let cells = vec![
        (Coord { t: 1, c: 2, o: 3, v: 4 }, tesseract::Cell::new()),
    ];
    p2p::gossip_boundary(&handle_b, cells).await;

    // Node A should receive boundary sync
    let received = tokio::time::timeout(
        Duration::from_secs(2),
        state_a.boundary_rx.recv(),
    ).await;

    assert!(received.is_ok(), "node A should receive boundary within 2s");
    let cells = received.unwrap().expect("channel closed");
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].0, Coord { t: 1, c: 2, o: 3, v: 4 });
}

#[tokio::test]
async fn ping_pong() {
    use tokio::net::TcpStream;

    let (_handle_a, _state_a) = p2p::start("node-a", "127.0.0.1:19300", &[])
        .await.expect("node A failed to start");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect directly and send a ping
    let mut stream = TcpStream::connect("127.0.0.1:19300").await.unwrap();
    let ping = p2p::Message::Ping { node_id: "test-client".to_string() };
    p2p::send_message(&mut stream, &ping).await.unwrap();

    // Should get a pong back
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        p2p::decode_message(&mut stream),
    ).await;

    assert!(response.is_ok(), "should get pong within 2s");
    let msg = response.unwrap().expect("no message");
    match msg {
        p2p::Message::Pong { node_id } => assert_eq!(node_id, "node-a"),
        other => panic!("expected Pong, got {:?}", other),
    }
}
