use crate::developer::Developer;

#[derive(Debug, Clone)]
/// Individual silver halide grain in a photographic emulsion
pub struct Halide {
    /// x position of grain in emulsion
    pub x: usize,
    /// y position of grain in emulsion
    pub y: usize,

    /// radius of grain in microns
    pub radius: f32,

    /// number of metalic silver atoms in each grain
    pub silver_count: usize,
    /// number of silver atoms needed to activate grain
    pub latent_threshold: usize,

    /// whether the grain has been activated
    pub activated: bool,
    /// sensitivity of the grain certain wavelengths of light
    pub spectral_sensitivity: f32,
    /// probability of a photon being absorbed by the grain
    pub absorption_probability: f32,

    /// fraction of maximum development achieved
    pub developed_fraction: f32,
}

impl Halide {
    pub fn expose(&mut self, intensity: f32, exposure_time: f32) {
        if self.activated {
            return;
        }

        let area = std::f32::consts::PI * (self.radius as f32).powi(2);
        let photon_count = (intensity * area * exposure_time) as usize;
        for _ in 0..photon_count {
            if rand::random::<f32>() < self.absorption_probability {
                self.silver_count += 1; // each photon that’s absorbed can form 1 Ag atom
                if self.silver_count >= self.latent_threshold {
                    self.activated = true;
                }
            }
        }
    }

    pub fn develop_grain(grain: &mut Halide, dev: &Developer, dt: f32) {
        // 'development_factor' goes from 0..1, representing how far the grain is to full silver
        let latent_ratio = (grain.silver_count as f32) / (grain.latent_threshold as f32);
        if latent_ratio > 1e-6 {
            // simulate some fraction of completion based on developer strength, latent ratio, and dt
            let rate = dev.strength * latent_ratio;
            // accumulate development in e.g. 'grain.developed_fraction' (0..1)
            grain.developed_fraction += rate * dt;
            if grain.developed_fraction > dev.max_development {
                grain.developed_fraction = dev.max_development;
            }
        }
    }
}
