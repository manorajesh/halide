use rayon::prelude::*;
use std::{ collections::HashSet, sync::{ Arc, Mutex } };
use rand::Rng;
use crate::halide::Halide;

pub struct Emulsion {
    pub grains: Vec<Halide>,
}

impl Emulsion {
    pub fn create_random_emulsion(width: u32, height: u32, num_grains: usize) -> Self {
        let emulsion = Arc::new(Mutex::new(Vec::with_capacity(num_grains)));
        // let grain_positions = Arc::new(Mutex::new(HashSet::new()));

        (0..num_grains).into_par_iter().for_each(|_| {
            let mut rng = rand::rng();
            let x = rng.random_range(0..width as usize);
            let y = rng.random_range(0..height as usize);

            let radius = rng.random_range(0.1..0.5);
            let latent_threshold = rng.random_range(5..20);
            let absorption_probability = rng.random_range(0.3..0.6);

            let halide = Halide {
                x,
                y,
                radius,
                silver_count: 0,
                latent_threshold,
                activated: false,
                spectral_sensitivity: 0.0,
                absorption_probability,
                developed_fraction: 0.0,
            };

            emulsion.lock().unwrap().push(halide);
        });
        let lock = emulsion.lock().unwrap();
        let grains = lock.clone();
        Self { grains }
    }

    pub fn render_emulsion(&self, width: u32, height: u32) -> image::RgbaImage {
        let mut output = image::RgbaImage::new(width, height);
        for pixel in output.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 255]);
        }

        for grain in self.grains.iter() {
            let gx = grain.x as i32;
            let gy = grain.y as i32;
            if gx < 0 || gy < 0 || gx >= (width as i32) || gy >= (height as i32) {
                continue;
            }

            // For a log-like final density (inverted):
            // D = A * log(1 + B * developed_fraction)
            let A = 0.5;
            let B = 10.0;
            let log_density = A * (1.0 + B * grain.developed_fraction).ln();

            // Convert to grayscale
            // If log_density ~0 => bright, if log_density is large => dark
            let intensity = (255.0 * (1.0 - log_density)).clamp(0.0, 255.0) as u8;

            output.put_pixel(
                gx as u32,
                gy as u32,
                image::Rgba([intensity, intensity, intensity, 255])
            );
        }
        output
    }
}
