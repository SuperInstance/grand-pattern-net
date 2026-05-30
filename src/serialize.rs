use crate::murmur::Murmur;
use crate::gossip::CellGraph;
use crate::vibe::Vibe;

/// Murmur wire format (32 bytes):
///   [0..16]  vibe dims as 16 x u8 (each f64 * 255, clamped)
///   [16..24] source_id as up to 8 bytes ASCII (null-padded)
///   [24]     level (u8)
///   [25]     ttl (u8)
///   [26]     hops (u8)
///   [27..31] timestamp as u32 (little-endian)
///   [31]     reserved (0)

pub fn serialize_murmur(murmur: &Murmur) -> [u8; 32] {
    let mut out = [0u8; 32];

    // Vibe as 16 x u8
    for i in 0..16 {
        out[i] = (murmur.vibe.dims[i].clamp(0.0, 1.0) * 255.0) as u8;
    }

    // source_id: up to 8 bytes
    let id_bytes = murmur.source_id.as_bytes();
    let id_len = id_bytes.len().min(8);
    out[16..16 + id_len].copy_from_slice(&id_bytes[..id_len]);

    out[24] = murmur.level;
    out[25] = murmur.ttl;
    out[26] = murmur.hops;

    let ts_bytes = (murmur.timestamp as u32).to_le_bytes();
    out[27] = ts_bytes[0];
    out[28] = ts_bytes[1];
    out[29] = ts_bytes[2];
    out[30] = ts_bytes[3];
    out[31] = 0;

    out
}

pub fn deserialize_murmur(data: &[u8; 32]) -> Option<Murmur> {
    let mut dims = [0.5f64; 16];
    for i in 0..16 {
        dims[i] = data[i] as f64 / 255.0;
    }

    let id_end = (16..24).find(|&i| data[i] == 0).unwrap_or(24);
    let source_id = std::str::from_utf8(&data[16..id_end]).unwrap_or("unknown").to_string();

    let level = data[24];
    let ttl = data[25];
    let hops = data[26];

    let timestamp = u32::from_le_bytes([data[27], data[28], data[29], data[30]]) as u64;

    Some(Murmur {
        source_id,
        vibe: Vibe { dims },
        level,
        ttl,
        hops,
        timestamp,
    })
}

/// Graph state binary format:
///   [0..4]   room_count (u32 LE)
///   [4..8]   edge_count (u32 LE)
///   [8..12]  tick_count (u32 LE)
///   Then per room: 8-byte id + 16-byte vibe = 24 bytes
///   Then per edge: 8-byte from + 8-byte to = 16 bytes
pub fn serialize_graph_state(graph: &CellGraph) -> Vec<u8> {
    let room_count = graph.rooms.len() as u32;
    let mut edge_count = 0u32;
    for neighbors in graph.edges.values() {
        edge_count += neighbors.len() as u32;
    }
    let tick_count = graph.current_tick() as u32;

    let mut buf = Vec::with_capacity(12 + graph.rooms.len() * 24 + edge_count as usize * 16);

    buf.extend_from_slice(&room_count.to_le_bytes());
    buf.extend_from_slice(&edge_count.to_le_bytes());
    buf.extend_from_slice(&tick_count.to_le_bytes());

    let mut room_ids: Vec<_> = graph.rooms.keys().collect();
    room_ids.sort();
    for rid in &room_ids {
        let room = graph.rooms.get(*rid).unwrap();
        let id_bytes = rid.as_bytes();
        let id_len = id_bytes.len().min(8);
        let mut id_buf = [0u8; 8];
        id_buf[..id_len].copy_from_slice(&id_bytes[..id_len]);
        buf.extend_from_slice(&id_buf);

        for i in 0..16 {
            buf.push((room.vibe.dims[i].clamp(0.0, 1.0) * 255.0) as u8);
        }
    }

    let mut edge_keys: Vec<_> = graph.edges.keys().collect();
    edge_keys.sort();
    for from_id in &edge_keys {
        let neighbors = graph.edges.get(*from_id).unwrap();
        let mut sorted_neighbors = neighbors.clone();
        sorted_neighbors.sort();
        for to_id in &sorted_neighbors {
            let from_bytes = from_id.as_bytes();
            let from_len = from_bytes.len().min(8);
            let mut from_buf = [0u8; 8];
            from_buf[..from_len].copy_from_slice(&from_bytes[..from_len]);
            buf.extend_from_slice(&from_buf);

            let to_bytes = to_id.as_bytes();
            let to_len = to_bytes.len().min(8);
            let mut to_buf = [0u8; 8];
            to_buf[..to_len].copy_from_slice(&to_bytes[..to_len]);
            buf.extend_from_slice(&to_buf);
        }
    }

    buf
}

pub fn deserialize_graph_state(data: &[u8]) -> CellGraph {
    if data.len() < 12 {
        return CellGraph::new();
    }

    let room_count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let edge_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let tick_count = u64::from(u32::from_le_bytes([data[8], data[9], data[10], data[11]]));

    let mut graph = CellGraph::new();
    graph.set_tick(tick_count);

    let mut offset = 12;

    for _ in 0..room_count {
        if offset + 24 > data.len() { break; }
        let id_end = (offset..offset + 8).find(|&i| data[i] == 0).unwrap_or(offset + 8);
        let room_id = std::str::from_utf8(&data[offset..id_end]).unwrap_or("?").to_string();

        let mut dims = [0.5f64; 16];
        for i in 0..16 {
            dims[i] = data[offset + 8 + i] as f64 / 255.0;
        }
        graph.add_room(&room_id, Some(Vibe { dims }));
        offset += 24;
    }

    for _ in 0..edge_count {
        if offset + 16 > data.len() { break; }
        let from_end = (offset..offset + 8).find(|&i| data[i] == 0).unwrap_or(offset + 8);
        let from_id = std::str::from_utf8(&data[offset..from_end]).unwrap_or("?").to_string();

        let to_end = (offset + 8..offset + 16).find(|&i| data[i] == 0).unwrap_or(offset + 16);
        let to_id = std::str::from_utf8(&data[offset + 8..to_end]).unwrap_or("?").to_string();

        graph.add_edge(&from_id, &to_id);
        offset += 16;
    }

    graph
}
