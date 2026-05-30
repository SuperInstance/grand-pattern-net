/// Vibe — 16-dimensional emotional/spectral descriptor.
/// Each dimension is an f64 in [0.0, 1.0].

pub const DIM_COUNT: usize = 16;

#[derive(Clone, Debug)]
pub struct Vibe {
    pub dims: [f64; DIM_COUNT],
}

impl Vibe {
    pub fn new() -> Self {
        Self { dims: [0.5; DIM_COUNT] }
    }

    pub fn from_vals(vals: [f64; DIM_COUNT]) -> Self {
        let mut dims = vals;
        for d in &mut dims {
            *d = d.clamp(0.0, 1.0);
        }
        Self { dims }
    }

    pub fn energy(&self) -> f64 {
        self.dims.iter().map(|d| d * d).sum::<f64>().sqrt()
    }

    pub fn distance(&self, other: &Vibe) -> f64 {
        self.dims.iter().zip(other.dims.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
    }

    pub fn groove_lock(&self, other: &Vibe) -> f64 {
        let dot: f64 = self.dims.iter().zip(other.dims.iter()).map(|(a, b)| a * b).sum();
        let se = self.energy();
        let oe = other.energy();
        if se == 0.0 || oe == 0.0 { return 0.0; }
        dot / (se * oe)
    }

    pub fn blend(&self, other: &Vibe, ratio: f64) -> Vibe {
        let r = ratio.clamp(0.0, 1.0);
        let mut dims = [0.0; DIM_COUNT];
        for i in 0..DIM_COUNT {
            dims[i] = self.dims[i] * (1.0 - r) + other.dims[i] * r;
        }
        Vibe { dims }
    }

    pub fn diffuse(&self, neighbors: &[Vibe], coeff: f64) -> Vibe {
        if neighbors.is_empty() {
            return self.clone();
        }
        let n = neighbors.len() as f64;
        let mut avg = [0.0; DIM_COUNT];
        for nb in neighbors {
            for i in 0..DIM_COUNT {
                avg[i] += nb.dims[i] / n;
            }
        }
        let mut dims = [0.0; DIM_COUNT];
        for i in 0..DIM_COUNT {
            dims[i] = self.dims[i] * (1.0 - coeff) + avg[i] * coeff;
        }
        Vibe { dims }
    }
}

impl Default for Vibe {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Vibe {
    fn eq(&self, other: &Self) -> bool {
        self.dims == other.dims
    }
}
