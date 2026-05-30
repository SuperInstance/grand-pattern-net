use crate::room::Room;
use crate::murmur::Murmur;
use crate::vibe::Vibe;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

/// CellGraph — rooms connected by edges, gossiping.
#[derive(Clone, Debug)]
pub struct CellGraph {
    pub rooms: HashMap<String, Room>,
    pub edges: HashMap<String, Vec<String>>,
    tick_count: u64,
}

impl CellGraph {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            edges: HashMap::new(),
            tick_count: 0,
        }
    }

    pub fn add_room(&mut self, room_id: &str, initial_vibe: Option<Vibe>) {
        let room = Room::new(room_id, initial_vibe);
        self.rooms.insert(room_id.to_string(), room);
        self.edges.entry(room_id.to_string()).or_default();
    }

    pub fn add_edge(&mut self, a: &str, b: &str) {
        self.edges.entry(a.to_string()).or_default().push(b.to_string());
        self.edges.entry(b.to_string()).or_default().push(a.to_string());
    }

    pub fn tick(&mut self) -> u64 {
        self.tick_count += 1;
        for room in self.rooms.values_mut() {
            room.tick();
        }
        self.tick_count
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_count
    }

    pub fn set_tick(&mut self, tick: u64) {
        self.tick_count = tick;
    }

    pub fn fleet_vibe(&self) -> Vibe {
        if self.rooms.is_empty() {
            return Vibe::new();
        }
        let n = self.rooms.len() as f64;
        let mut avg = [0.0; 16];
        for room in self.rooms.values() {
            for i in 0..16 {
                avg[i] += room.vibe.dims[i] / n;
            }
        }
        Vibe { dims: avg }
    }

    pub fn diffuse_all(&mut self, coeff: f64) {
        let new_vibes: HashMap<String, Vibe> = self.edges.iter().map(|(rid, neighbors)| {
            let room = self.rooms.get(rid).unwrap();
            let nbs: Vec<Vibe> = neighbors.iter()
                .filter_map(|n| self.rooms.get(n))
                .map(|r| r.vibe.clone())
                .collect();
            (rid.clone(), room.vibe.diffuse(&nbs, coeff))
        }).collect();
        for (rid, vibe) in new_vibes {
            if let Some(r) = self.rooms.get_mut(&rid) {
                r.vibe = vibe;
            }
        }
    }

    /// Collect murmurs for all rooms (for gossip broadcast).
    pub fn collect_murmurs(&self) -> Vec<Murmur> {
        self.rooms.values().map(|r| {
            Murmur::new(&r.room_id, &r.vibe, 0, 5, self.tick_count)
        }).collect()
    }

    /// Integrate a murmur from a peer into the graph.
    pub fn integrate_murmur(&mut self, murmur: &Murmur) {
        if murmur.is_expired(self.tick_count) {
            return;
        }
        if let Some(room) = self.rooms.get_mut(&murmur.source_id) {
            // Blend peer's vibe into existing room
            room.vibe = room.vibe.blend(&murmur.vibe, 0.3);
        } else {
            // Unknown room — create it from murmur data
            self.add_room(&murmur.source_id, Some(murmur.vibe.clone()));
        }
    }
}

impl Default for CellGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// GossipNode — a networked CellGraph that communicates via UDP.
pub struct GossipNode {
    pub id: usize,
    pub graph: CellGraph,
    pub port: u16,
    pub peers: Vec<SocketAddr>,
    socket: Option<UdpSocket>,
    seen_murmurs: HashMap<(String, u64), bool>, // dedup: (source_id, timestamp)
}

impl GossipNode {
    pub fn new(id: usize, port: u16) -> Self {
        Self {
            id,
            graph: CellGraph::new(),
            port,
            peers: Vec::new(),
            socket: None,
            seen_murmurs: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, addr: SocketAddr) {
        if !self.peers.contains(&addr) {
            self.peers.push(addr);
        }
    }

    pub fn bind(&mut self) -> std::io::Result<()> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", self.port))?;
        socket.set_nonblocking(true)?;
        self.socket = Some(socket);
        Ok(())
    }

    pub fn tick(&mut self) -> u64 {
        self.graph.tick()
    }

    pub fn broadcast_murmurs(&self) -> std::io::Result<usize> {
        let socket = self.socket.as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "not bound"))?;
        let murmurs = self.graph.collect_murmurs();
        let mut sent = 0;
        for murmur in &murmurs {
            let data = crate::serialize::serialize_murmur(murmur);
            for peer in &self.peers {
                if socket.send_to(&data, peer).is_ok() {
                    sent += 1;
                }
            }
        }
        Ok(sent)
    }

    pub fn receive_murmurs(&mut self) -> Vec<Murmur> {
        let socket = match self.socket.as_ref() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let mut received = Vec::new();
        let mut buf = [0u8; 256];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, _addr)) => {
                    if len >= 32 {
                        let mut data = [0u8; 32];
                        data.copy_from_slice(&buf[..32]);
                        if let Some(murmur) = crate::serialize::deserialize_murmur(&data) {
                            let key = (murmur.source_id.clone(), murmur.timestamp);
                            if !self.seen_murmurs.contains_key(&key) {
                                self.seen_murmurs.insert(key, true);
                                received.push(murmur);
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        received
    }

    pub fn integrate_murmur(&mut self, murmur: Murmur) {
        self.graph.integrate_murmur(&murmur);
    }

    /// Number of unique murmurs seen (for testing dedup).
    pub fn seen_count(&self) -> usize {
        self.seen_murmurs.len()
    }
}
