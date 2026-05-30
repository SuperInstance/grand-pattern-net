//! grand-pattern-net — Networking layer for the Grand Pattern.
//!
//! Rooms gossip over the network via UDP multicast, with TCP fallback
//! for reliable delivery. Pure Rust, std::net only (zero external deps).

pub mod gossip;
pub mod murmur;
pub mod room;
pub mod tcp_transport;
pub mod discovery;
pub mod serialize;
pub mod coord;
pub mod vibe;

pub use gossip::{CellGraph, GossipNode};
pub use murmur::Murmur;
pub use room::Room;
pub use tcp_transport::TcpGossip;
pub use vibe::Vibe;
