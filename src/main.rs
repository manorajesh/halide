use std::{ collections::HashSet, sync::{ Arc, Mutex } };

use rayon::prelude::*;

#[derive(Debug, Clone)]
/// Individual silver halide grain in a photographic emulsion
struct Halide {
    /// x position of grain in emulsion
    x: usize,
    /// y position of grain in emulsion
    y: usize,

    /// radius of grain in microns
    radius: f32,

    /// number of metalic silver atoms in each grain
    silver_count: usize,
    /// number of silver atoms needed to activate grain
    latent_threshold: usize,

    /// whether the grain has been activated
    activated: bool,
    /// sensitivity of the grain certain wavelengths of light
    spectral_sensitivity: f32,
    /// probability of a photon being absorbed by the grain
    absorption_probability: f32,
}

impl Halide {
    fn expose(&mut self, intensity: f32, exposure_time: f32) {
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

    fn develop(&self) -> f32 {
        let dev_amount = (self.silver_count as f32) / (self.latent_threshold as f32);
        dev_amount
    }
}

use rand::Rng;

fn create_random_emulsion(width: u32, height: u32, num_grains: usize) -> Vec<Halide> {
    let emulsion = Arc::new(Mutex::new(Vec::with_capacity(num_grains)));
    let grain_positions = Arc::new(Mutex::new(HashSet::new()));

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
        };

        if grain_positions.lock().unwrap().insert((x, y)) {
            emulsion.lock().unwrap().push(halide);
        }
    });
    let lock = emulsion.lock().unwrap();
    lock.clone()
}

fn render_emulsion(emulsion: &Vec<Halide>, width: u32, height: u32) -> image::RgbaImage {
    let mut output = image::RgbaImage::new(width, height);
    for pixel in output.pixels_mut() {
        *pixel = image::Rgba([255, 255, 255, 255]);
    }

    for grain in emulsion {
        // Mark a small circle
        let r2 = (grain.radius * grain.radius) as i32;
        let gx = grain.x as i32;
        let gy = grain.y as i32;
        let radius = grain.radius as i32;
        let development = grain.develop();

        for dy in -radius as i32..=radius as i32 {
            for dx in -radius as i32..=radius as i32 {
                if dx * dx + dy * dy <= r2 {
                    let px = gx + dx;
                    let py = gy + dy;
                    if px >= 0 && py >= 0 && px < (width as i32) && py < (height as i32) {
                        let intensity = (255.0 / development).round() as u8;
                        output.put_pixel(
                            px as u32,
                            py as u32,
                            image::Rgba([intensity, intensity, intensity, 255])
                        );
                    }
                }
            }
        }
    }
    output
}

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Creating emulsion");

    // open input image
    let image = image::open("input.jpeg").unwrap();
    let image = image.to_luma16();
    let (width, height) = image.dimensions();

    let num_grains = 10000000;
    let mut emulsion = create_random_emulsion(width, height, num_grains);

    // expose emulsion to image
    tracing::info!("Exposing emulsion to image");
    emulsion.par_iter_mut().for_each(|grain| {
        let pixel_val = image.get_pixel(grain.x as u32, grain.y as u32).0[0];
        let intensity = (pixel_val as f32) / (u16::MAX as f32);
        grain.expose(intensity, 500.0);
    });

    // save activated grains to output image
    tracing::info!("Saving activated grains to negative image");
    let output = render_emulsion(&emulsion, width, height);
    output.save("negative.png").unwrap();
}
