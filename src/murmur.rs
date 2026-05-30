use crate::vibe::Vibe;

/// A gossip packet carrying a vibe snapshot.
#[derive(Clone, Debug)]
pub struct Murmur {
    pub source_id: String,
    pub vibe: Vibe,
    pub level: u8,      // 0=neighbor, 1=zone, 2=fleet
    pub ttl: u8,
    pub hops: u8,
    pub timestamp: u64, // monotonic tick counter
}

impl Murmur {
    pub fn new(source_id: &str, vibe: &Vibe, level: u8, ttl: u8, timestamp: u64) -> Self {
        Self {
            source_id: source_id.to_string(),
            vibe: vibe.clone(),
            level,
            ttl,
            hops: 0,
            timestamp,
        }
    }

    pub fn is_expired(&self, current_tick: u64) -> bool {
        self.ttl == 0 || current_tick.saturating_sub(self.timestamp) > 60
    }

    pub fn decay(&self) -> Murmur {
        Murmur {
            source_id: self.source_id.clone(),
            vibe: self.vibe.clone(),
            level: self.level,
            ttl: self.ttl.saturating_sub(1),
            hops: self.hops + 1,
            timestamp: self.timestamp,
        }
    }
}
