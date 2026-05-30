# grand-pattern-net

Networking layer for the Grand Pattern. Rooms gossip over the network.

## Features

- **UDP Gossip Protocol** — Multicast-based murmur propagation between nodes
- **TCP Transport** — Reliable framed delivery for murmur exchange
- **Peer Discovery** — Multicast announcement and discovery protocol
- **Binary Serialization** — Compact 32-byte murmur encoding, deterministic graph state wire format
- **Distributed Tick Coordination** — All nodes tick, exchange murmurs, integrate
- **Zero external dependencies** — Pure Rust, `std::net` only

## Architecture

```
┌─────────────┐     UDP Multicast     ┌─────────────┐
│  GossipNode │◄──────────────────────►│  GossipNode │
│  (CellGraph)│                        │  (CellGraph)│
└──────┬──────┘                        └──────┬──────┘
       │                                      │
       │ TCP (reliable)                       │ TCP
       ▼                                      ▼
┌──────────────┐                      ┌──────────────┐
│  TcpGossip   │                      │  TcpGossip   │
└──────────────┘                      └──────────────┘
```

## Core Types

### Vibe (16-dimensional)
Each room carries a 16-dim vibe descriptor (dark/bright/warm/harsh/dense/sparse/fast/slow/dry/wet/tight/loose/forward/distant/smooth/rough).

### Murmur (gossip packet)
A 32-byte compact binary packet carrying a room's vibe snapshot with TTL, hop count, and source ID.

### CellGraph
Composes rooms + edges. Supports tick, diffusion, fleet vibe aggregation, and murmur integration.

### GossipNode
Networked CellGraph with UDP socket, peer list, deduplication, and broadcast/receive.

## Usage

```rust
use grand_pattern_net::*;

// Create nodes
let mut node = GossipNode::new(1, 9000);
node.graph.add_room("kitchen", Some(Vibe::new()));
node.graph.add_room("studio", Some(Vibe::new()));
node.graph.add_edge("kitchen", "studio");

// Tick and gossip
node.tick();
let murmurs = node.graph.collect_murmurs();
```

## Running Tests

```bash
cargo test
```

30 tests covering gossip, serialization, TCP transport, discovery, coordination, partition recovery, and performance (100 nodes).
