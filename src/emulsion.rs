use rayon::prelude::*;
use std::{ sync::{ Arc, Mutex } };
use rand::Rng;
use crate::halide::Halide;

pub struct Emulsion {
    pub grains: Vec<Halide>,
}

impl Emulsion {
    pub fn from(grains: Vec<Halide>) -> Self {
        Self { grains }
    }

    pub fn create_random_emulsion(width: u32, height: u32, num_grains: usize) -> Self {
        let emulsion = Arc::new(Mutex::new(Vec::with_capacity(num_grains)));

        (0..num_grains).into_par_iter().for_each(|_| {
            let mut rng = rand::rng();
            let x = rng.random_range(0..width as usize);
            let y = rng.random_range(0..height as usize);

            let halide = Halide::new(x, y);

            emulsion.lock().unwrap().push(halide);
        });
        let lock = emulsion.lock().unwrap();
        let grains = lock.clone();
        Self { grains }
    }

    /// Simple H&D curve:
    ///     D = D_min + gamma * log10( (exposure + E0) / E0 )
    /// Then clamp to [D_min, D_max].
    ///
    /// - `developed_frac` in [0..1] we treat as "exposure" for simplicity.
    fn film_density(developed_frac: f32) -> f32 {
        // Constants you can tweak:
        let d_min = 0.05; // base + fog density
        let d_max = 2.5; // "shoulder" limit
        let gamma = 0.8; // slope in the linear region
        let e0 = 0.02; // reference offset to avoid log(0)

        // Interpret developed_fraction as a rough 'exposure' measure:
        let e = developed_frac + e0; // shift by E0
        let mut density = d_min + gamma * (e / e0).log10();

        // Clamp to [d_min, d_max]
        if density < d_min {
            density = d_min;
        } else if density > d_max {
            density = d_max;
        }
        density
    }

    pub fn render_emulsion(&self, width: u32, height: u32) -> image::RgbaImage {
        let mut output = image::RgbaImage::new(width, height);

        // Start white
        for pixel in output.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 255]);
        }

        for grain in self.grains.iter() {
            let gx = grain.x as i32;
            let gy = grain.y as i32;
            if gx < 0 || gy < 0 || gx >= (width as i32) || gy >= (height as i32) {
                continue;
            }

            // Compute an H&D-like density from [d_min..d_max]
            let density = Self::film_density(grain.developed_fraction);

            // Map density -> pixel intensity in [0..255], invert so higher density = darker
            // We'll re-use the same d_min, d_max as in film_density for consistency
            let (d_min, d_max) = (0.05, 2.5);
            let norm = (density - d_min) / (d_max - d_min); // in [0..1]
            // invert: norm=0 => bright, norm=1 => black
            let intensity = (255.0 * (1.0 - norm)).clamp(0.0, 255.0) as u8;

            output.put_pixel(
                gx as u32,
                gy as u32,
                image::Rgba([intensity, intensity, intensity, 255])
            );
        }

        output
    }
}
