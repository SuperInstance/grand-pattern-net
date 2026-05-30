use grand_pattern_net::*;
use grand_pattern_net::serialize::*;
use grand_pattern_net::coord::*;
use grand_pattern_net::discovery::*;
use grand_pattern_net::tcp_transport::*;
use std::net::SocketAddr;

fn make_test_vibe(val: f64) -> vibe::Vibe {
    let mut dims = [0.5f64; 16];
    dims[0] = val;
    dims[1] = 1.0 - val;
    vibe::Vibe { dims }
}

// ---- Test 1: Gossip node creation ----
#[test]
fn test_gossip_node_creation() {
    let node = gossip::GossipNode::new(1, 9090);
    assert_eq!(node.id, 1);
    assert_eq!(node.port, 9090);
    assert!(node.peers.is_empty());
    assert!(node.graph.rooms.is_empty());
}

// ---- Test 2: Add peer ----
#[test]
fn test_add_peer() {
    let mut node = gossip::GossipNode::new(0, 9000);
    let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();
    node.add_peer(addr);
    assert_eq!(node.peers.len(), 1);
    assert_eq!(node.peers[0], addr);
    node.add_peer(addr);
    assert_eq!(node.peers.len(), 1);
}

// ---- Test 3: Tick updates graph ----
#[test]
fn test_tick_updates_graph() {
    let mut node = gossip::GossipNode::new(0, 9000);
    node.graph.add_room("r1", None);
    assert_eq!(node.graph.current_tick(), 0);
    let t = node.tick();
    assert_eq!(t, 1);
    assert_eq!(node.graph.current_tick(), 1);
}

// ---- Test 4: Serialize/deserialize murmur roundtrip ----
#[test]
fn test_serialize_deserialize_murmur() {
    let v = make_test_vibe(0.8);
    let murmur = murmur::Murmur::new("room1", &v, 1, 5, 42);
    let data = serialize_murmur(&murmur);
    assert_eq!(data.len(), 32);
    let restored = deserialize_murmur(&data).unwrap();
    assert_eq!(restored.source_id, "room1");
    assert_eq!(restored.level, 1);
    assert_eq!(restored.ttl, 5);
    assert_eq!(restored.timestamp, 42);
    for i in 0..16 {
        let diff = (restored.vibe.dims[i] - murmur.vibe.dims[i]).abs();
        assert!(diff < 0.01, "dim {} differs by {}", i, diff);
    }
}

// ---- Test 5: Serialize/deserialize graph roundtrip ----
#[test]
fn test_serialize_deserialize_graph() {
    let mut graph = CellGraph::new();
    graph.add_room("alpha", Some(make_test_vibe(0.3)));
    graph.add_room("beta", Some(make_test_vibe(0.7)));
    graph.add_edge("alpha", "beta");
    graph.set_tick(10);

    let data = serialize_graph_state(&graph);
    let restored = deserialize_graph_state(&data);

    assert_eq!(restored.rooms.len(), 2);
    assert_eq!(restored.current_tick(), 10);
    assert!(restored.rooms.contains_key("alpha"));
    assert!(restored.rooms.contains_key("beta"));
}

// ---- Test 6: Integrate murmur from peer ----
#[test]
fn test_integrate_murmur() {
    let mut graph = CellGraph::new();
    graph.add_room("room1", Some(make_test_vibe(0.5)));

    let murmur = murmur::Murmur::new("room1", &make_test_vibe(0.9), 0, 5, 0);
    graph.integrate_murmur(&murmur);

    let room = graph.rooms.get("room1").unwrap();
    assert!((room.vibe.dims[0] - 0.62).abs() < 0.01);
}

// ---- Test 7: Integrate ignores expired murmurs ----
#[test]
fn test_integrate_ignores_expired() {
    let mut graph = CellGraph::new();
    graph.set_tick(100);
    graph.add_room("room1", Some(make_test_vibe(0.5)));

    let murmur = murmur::Murmur::new("room1", &make_test_vibe(0.9), 0, 5, 0);
    graph.integrate_murmur(&murmur);

    let room = graph.rooms.get("room1").unwrap();
    assert!((room.vibe.dims[0] - 0.5).abs() < 0.01);
}

// ---- Test 8: Integrate handles unknown room gracefully ----
#[test]
fn test_integrate_unknown_room() {
    let mut graph = CellGraph::new();
    assert!(graph.rooms.is_empty());

    let murmur = murmur::Murmur::new("newroom", &make_test_vibe(0.8), 0, 5, 0);
    graph.integrate_murmur(&murmur);

    assert!(graph.rooms.contains_key("newroom"));
    let room = graph.rooms.get("newroom").unwrap();
    assert!((room.vibe.dims[0] - 0.8).abs() < 0.01);
}

// ---- Test 9: Two nodes gossip to each other ----
#[test]
fn test_two_nodes_gossip() {
    let mut n1 = gossip::GossipNode::new(1, 0);
    let mut n2 = gossip::GossipNode::new(2, 0);

    n1.graph.add_room("room1", Some(make_test_vibe(0.2)));
    n2.graph.add_room("room2", Some(make_test_vibe(0.8)));

    let mut nodes = vec![n1, n2];
    coord::coordinate_tick(&mut nodes, 0.5);

    assert!(nodes[0].graph.rooms.contains_key("room2"));
    assert!(nodes[1].graph.rooms.contains_key("room1"));
}

// ---- Test 10: Three nodes gossip in triangle ----
#[test]
fn test_three_nodes_triangle() {
    let mut nodes: Vec<gossip::GossipNode> = (0..3).map(|i| {
        let mut n = gossip::GossipNode::new(i, 0);
        n.graph.add_room(&format!("r{}", i), Some(make_test_vibe(i as f64 * 0.3)));
        n
    }).collect();

    coordinate_tick(&mut nodes, 0.5);

    for node in &nodes {
        assert!(node.graph.rooms.contains_key("r0"));
        assert!(node.graph.rooms.contains_key("r1"));
        assert!(node.graph.rooms.contains_key("r2"));
    }
}

// ---- Test 11: Discovery finds peers (mock) ----
#[test]
fn test_discovery_mock() {
    let peers = mock_discover_peers(5, 8000);
    assert_eq!(peers.len(), 5);
    for (i, p) in peers.iter().enumerate() {
        assert_eq!(p.port(), 8000 + i as u16);
    }
}

// ---- Test 12: TCP transport sends/receives ----
#[test]
fn test_tcp_transport() {
    let mut server = TcpGossip::bind("127.0.0.1:0").unwrap();
    let server_addr = server.listener.local_addr().unwrap();

    let mut client = TcpGossip::bind("127.0.0.1:0").unwrap();
    client.connect(&server_addr.to_string()).unwrap();

    server.accept_connections();
    assert!(server.connection_count() >= 1);

    let murmur = murmur::Murmur::new("test", &make_test_vibe(0.5), 0, 5, 1);
    let sent = client.send_murmur(&murmur);
    assert_eq!(sent, 1);
}

// ---- Test 13: Distributed tick coordination ----
#[test]
fn test_coordinate_tick() {
    let mut nodes: Vec<gossip::GossipNode> = (0..4).map(|i| {
        let mut n = gossip::GossipNode::new(i, 0);
        n.graph.add_room(&format!("r{}", i), Some(make_test_vibe(0.5)));
        n
    }).collect();

    coordinate_tick(&mut nodes, 0.5);
    for node in &nodes {
        assert_eq!(node.graph.current_tick(), 1);
    }
}

// ---- Test 14: Conservation across distributed nodes ----
#[test]
fn test_conservation() {
    let nodes: Vec<gossip::GossipNode> = (0..3).map(|i| {
        let mut n = gossip::GossipNode::new(i, 0);
        n.graph.add_room(&format!("r{}", i), Some(make_test_vibe(0.5)));
        n
    }).collect();

    assert!(check_conservation(&nodes, 1.0));
}

// ---- Test 15: Node with no peers handles gracefully ----
#[test]
fn test_no_peers_graceful() {
    let mut node = gossip::GossipNode::new(0, 0);
    node.graph.add_room("solo", Some(make_test_vibe(0.5)));
    node.tick();
    assert_eq!(node.graph.current_tick(), 1);
}

// ---- Test 16: Duplicate murmur detection ----
#[test]
fn test_duplicate_murmur_detection() {
    let mut node = gossip::GossipNode::new(0, 0);
    let murmur = murmur::Murmur::new("src", &make_test_vibe(0.5), 0, 5, 1);

    node.integrate_murmur(murmur.clone());
    node.integrate_murmur(murmur.clone());
    assert!(node.graph.rooms.contains_key("src"));
    assert_eq!(node.graph.rooms.len(), 1);
}

// ---- Test 17: Murmur ordering (newer overwrites older) ----
#[test]
fn test_murmur_ordering() {
    let mut graph = CellGraph::new();
    graph.add_room("r1", Some(make_test_vibe(0.1)));

    let old = murmur::Murmur::new("r1", &make_test_vibe(0.2), 0, 5, 0);
    graph.integrate_murmur(&old);
    let after_old = graph.rooms.get("r1").unwrap().vibe.dims[0];

    let new = murmur::Murmur::new("r1", &make_test_vibe(0.9), 0, 5, 1);
    graph.integrate_murmur(&new);
    let after_new = graph.rooms.get("r1").unwrap().vibe.dims[0];

    assert!(after_new > after_old);
}

// ---- Test 18: Network partition recovery ----
#[test]
fn test_partition_recovery() {
    let mut n1 = gossip::GossipNode::new(1, 0);
    let mut n2 = gossip::GossipNode::new(2, 0);

    n1.graph.add_room("a", Some(make_test_vibe(0.1)));
    n2.graph.add_room("b", Some(make_test_vibe(0.9)));

    // Partition: tick independently
    n1.tick();
    n2.tick();

    // Recovery: coordinate again
    let mut nodes = vec![n1, n2];
    coordinate_tick(&mut nodes, 0.5);
    assert!(nodes[0].graph.rooms.contains_key("b"));
    assert!(nodes[1].graph.rooms.contains_key("a"));
}

// ---- Test 19: Large message handling ----
#[test]
fn test_large_graph_serialization() {
    let mut graph = CellGraph::new();
    for i in 0..100 {
        graph.add_room(&format!("r{:03}", i), Some(make_test_vibe((i % 10) as f64 / 10.0)));
        if i > 0 {
            graph.add_edge(&format!("r{:03}", i - 1), &format!("r{:03}", i));
        }
    }
    graph.set_tick(50);

    let data = serialize_graph_state(&graph);
    let restored = deserialize_graph_state(&data);

    assert_eq!(restored.rooms.len(), 100);
    assert_eq!(restored.current_tick(), 50);
}

// ---- Test 20: Concurrent send/receive (via coordinated tick) ----
#[test]
fn test_concurrent_send_receive() {
    let mut nodes: Vec<gossip::GossipNode> = (0..5).map(|i| {
        let mut n = gossip::GossipNode::new(i, 0);
        n.graph.add_room(&format!("r{}", i), Some(make_test_vibe(i as f64 / 5.0)));
        n
    }).collect();

    for _ in 0..10 {
        coordinate_tick(&mut nodes, 0.5);
    }
    for node in &nodes {
        assert_eq!(node.graph.current_tick(), 10);
        assert!(node.graph.rooms.len() >= 5);
    }
}

// ---- Test 21: Node joins mid-simulation ----
#[test]
fn test_node_joins_mid_simulation() {
    let mut nodes: Vec<gossip::GossipNode> = (0..2).map(|i| {
        let mut n = gossip::GossipNode::new(i + 1, 0);
        n.graph.add_room(&['a', 'b'][i].to_string(), Some(make_test_vibe(0.3 + i as f64 * 0.4)));
        n
    }).collect();

    coordinate_tick(&mut nodes, 0.5);

    // New node joins
    let mut n3 = gossip::GossipNode::new(3, 0);
    n3.graph.add_room("c", Some(make_test_vibe(0.5)));
    nodes.push(n3);

    coordinate_tick(&mut nodes, 0.5);

    assert!(nodes[0].graph.rooms.contains_key("c"));
    assert!(nodes[2].graph.rooms.contains_key("a"));
    assert!(nodes[2].graph.rooms.contains_key("b"));
}

// ---- Test 22: Node leaves mid-simulation ----
#[test]
fn test_node_leaves_mid_simulation() {
    let mut nodes: Vec<gossip::GossipNode> = (0..3).map(|i| {
        let mut n = gossip::GossipNode::new(i + 1, 0);
        n.graph.add_room(&['a', 'b', 'c'][i].to_string(), Some(make_test_vibe(0.3 + i as f64 * 0.2)));
        n
    }).collect();

    coordinate_tick(&mut nodes, 0.5);

    // Node 3 "leaves"
    nodes.pop();

    coordinate_tick(&mut nodes, 0.5);
    assert!(nodes[0].graph.rooms.contains_key("c"));
}

// ---- Test 23: Performance: 100 nodes gossip ----
#[test]
fn test_100_nodes_performance() {
    let mut nodes: Vec<gossip::GossipNode> = (0..100).map(|i| {
        let mut n = gossip::GossipNode::new(i, 0);
        n.graph.add_room(&format!("r{}", i), Some(make_test_vibe((i % 10) as f64 / 10.0)));
        n
    }).collect();

    let start = std::time::Instant::now();
    coordinate_tick(&mut nodes, 0.5);
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() < 5000, "Took {}ms", elapsed.as_millis());
    let avg_rooms: f64 = nodes.iter().map(|n| n.graph.rooms.len() as f64).sum::<f64>() / 100.0;
    assert!(avg_rooms >= 50.0, "Avg rooms: {}", avg_rooms);
}

// ---- Test 24: Compression reduces message size ----
#[test]
fn test_murmur_compact_size() {
    let murmur = murmur::Murmur::new("room1", &make_test_vibe(0.5), 0, 5, 100);
    let data = serialize_murmur(&murmur);
    assert_eq!(data.len(), 32);
    assert!(data.len() < 64);
}

// ---- Test 25: All operations deterministic ----
#[test]
fn test_deterministic_serialization() {
    let mut graph = CellGraph::new();
    graph.add_room("z", Some(make_test_vibe(0.1)));
    graph.add_room("a", Some(make_test_vibe(0.9)));
    graph.add_edge("z", "a");
    graph.set_tick(42);

    let data1 = serialize_graph_state(&graph);
    let data2 = serialize_graph_state(&graph);
    assert_eq!(data1, data2);

    let restored = deserialize_graph_state(&data1);
    assert_eq!(restored.rooms.len(), 2);
    assert_eq!(restored.current_tick(), 42);
}

// ---- Test 26: Murmur decay ----
#[test]
fn test_murmur_decay() {
    let m = murmur::Murmur::new("x", &make_test_vibe(0.5), 0, 3, 0);
    let d = m.decay();
    assert_eq!(d.ttl, 2);
    assert_eq!(d.hops, 1);
}

// ---- Test 27: Murmur expiry ----
#[test]
fn test_murmur_expiry() {
    let m = murmur::Murmur::new("x", &make_test_vibe(0.5), 0, 0, 0);
    assert!(m.is_expired(0));

    let m2 = murmur::Murmur::new("x", &make_test_vibe(0.5), 0, 5, 0);
    assert!(!m2.is_expired(1));
    assert!(m2.is_expired(61));
}

// ---- Test 28: Vibe operations ----
#[test]
fn test_vibe_operations() {
    let v1 = make_test_vibe(0.0);
    let v2 = make_test_vibe(1.0);

    let blended = v1.blend(&v2, 0.5);
    assert!((blended.dims[0] - 0.5).abs() < 0.01);
    assert!((blended.dims[1] - 0.5).abs() < 0.01);

    let dist = v1.distance(&v2);
    assert!(dist > 0.0);

    let energy = v1.energy();
    assert!(energy > 0.0);
}

// ---- Test 29: Graph diffusion ----
#[test]
fn test_graph_diffusion() {
    let mut graph = CellGraph::new();
    graph.add_room("a", Some(make_test_vibe(0.0)));
    graph.add_room("b", Some(make_test_vibe(1.0)));
    graph.add_edge("a", "b");

    graph.diffuse_all(0.5);

    let ra = graph.rooms.get("a").unwrap();
    let rb = graph.rooms.get("b").unwrap();
    assert!(ra.vibe.dims[0] > 0.0);
    assert!(rb.vibe.dims[0] < 1.0);
}

// ---- Test 30: Room tick perception ----
#[test]
fn test_room_tick() {
    let mut room = room::Room::new("test", Some(make_test_vibe(0.5)));
    room.tick();
    room.tick();
    room.tick();
    assert!(!room.perceptions.is_empty());
}
