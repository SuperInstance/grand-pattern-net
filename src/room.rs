use crate::vibe::Vibe;

/// Room — a single cell in the Grand Pattern.
/// Composes Vibe + simple prediction + murmur buffer.
#[derive(Clone, Debug)]
pub struct Room {
    pub room_id: String,
    pub vibe: Vibe,
    last_prediction: Vibe,
    pub perceptions: Vec<Vibe>,
}

impl Room {
    pub fn new(room_id: &str, initial_vibe: Option<Vibe>) -> Self {
        Self {
            room_id: room_id.to_string(),
            vibe: initial_vibe.unwrap_or_else(Vibe::new),
            last_prediction: Vibe::new(),
            perceptions: Vec::new(),
        }
    }

    pub fn perceive(&mut self, vibe: Option<&Vibe>) {
        let v = vibe.unwrap_or(&self.vibe);
        self.perceptions.push(v.clone());
        if self.perceptions.len() > 100 {
            self.perceptions.remove(0);
        }
    }

    pub fn predict(&self) -> Vibe {
        if self.perceptions.is_empty() {
            return Vibe::new();
        }
        let n = self.perceptions.len().min(10);
        let recent = &self.perceptions[self.perceptions.len() - n..];
        let len = recent.len() as f64;
        let mut avg = [0.0; 16];
        for v in recent {
            for i in 0..16 {
                avg[i] += v.dims[i] / len;
            }
        }
        Vibe { dims: avg }
    }

    pub fn surprise(&self, observed: Option<&Vibe>) -> f64 {
        let v = observed.unwrap_or(&self.vibe);
        let pred = self.predict();
        1.0 - pred.groove_lock(v).clamp(0.0, 1.0)
    }

    pub fn tick(&mut self) {
        self.perceive(None);
        self.last_prediction = self.predict();
    }

    pub fn diffuse(&mut self, neighbors: &[&Room], coeff: f64) {
        let vibes: Vec<Vibe> = neighbors.iter().map(|r| r.vibe.clone()).collect();
        self.vibe = self.vibe.diffuse(&vibes, coeff);
    }
}
