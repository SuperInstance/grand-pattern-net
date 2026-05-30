use crate::gossip::GossipNode;

/// Distributed tick coordination — all nodes tick, exchange murmurs, integrate.
/// Tolerance controls how much drift in fleet vibe is acceptable.

pub fn coordinate_tick(nodes: &mut [GossipNode], _tolerance: f64) {
    // Phase 1: All nodes tick
    for node in nodes.iter_mut() {
        node.tick();
    }

    // Phase 2: Collect murmurs from all nodes
    let all_murmurs: Vec<_> = nodes.iter()
        .flat_map(|n| n.graph.collect_murmurs())
        .collect();

    // Phase 3: Each node integrates all murmurs (except its own)
    for node in nodes.iter_mut() {
        for murmur in &all_murmurs {
            if murmur.source_id != format!("node-{}", node.id) {
                node.integrate_murmur(murmur.clone());
            }
        }
    }
}

/// Check that fleet vibe is roughly conserved across distributed nodes.
pub fn check_conservation(nodes: &[GossipNode], tolerance: f64) -> bool {
    if nodes.len() < 2 {
        return true;
    }
    let vibes: Vec<_> = nodes.iter().map(|n| n.graph.fleet_vibe()).collect();
    for i in 0..vibes.len() {
        for j in (i + 1)..vibes.len() {
            if vibes[i].distance(&vibes[j]) > tolerance {
                return false;
            }
        }
    }
    true
}
