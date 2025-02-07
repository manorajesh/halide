use rayon::prelude::*;

/// Individual silver halide grain in a photographic emulsion
struct Halide {
    /// x position of grain in emulsion
    x: usize,
    /// y position of grain in emulsion
    y: usize,

    /// radius of grain in microns
    radius: usize,

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
    fn new(x: usize, y: usize) -> Self {
        Halide {
            x,
            y,
            radius: 1,
            silver_count: 0,
            latent_threshold: 100,
            activated: false,
            spectral_sensitivity: 0.0,
            absorption_probability: 0.1,
        }
    }

    fn expose(&mut self, intensity: f32, exposure_time: f32) {
        if self.activated {
            return;
        }

        let area = std::f32::consts::PI * (self.radius as f32).powi(2);
        let photon_count = (intensity * area * exposure_time) as usize;
        let mut reduced_silver_atoms = 0;
        for _ in 0..photon_count {
            if rand::random::<f32>() < self.absorption_probability {
                reduced_silver_atoms += 1; // each photon that’s absorbed can form 1 Ag atom
            }
            if reduced_silver_atoms >= self.latent_threshold {
                self.activated = true;
                break;
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Creating emulsion");
    let mut emulsion = Vec::new();
    let num_grains = 1000;
    for x in 0..num_grains {
        for y in 0..num_grains {
            emulsion.push(Halide::new(x, y));
        }
    }

    // open input image
    let image = image::open("input.png").unwrap();
    let image = image.to_luma16();
    let (width, height) = image.dimensions();

    // expose emulsion to image
    tracing::info!("Exposing emulsion to image");
    emulsion.par_iter_mut().for_each(|grain| {
        let x = (grain.x * (width as usize)) / num_grains;
        let y = (grain.y * (height as usize)) / num_grains;
        let intensity =
            ((image.get_pixel(x as u32, y as u32).0[0] as f32) / (u16::MAX as f32)) * 1000.0;
        // println!("Exposing grain at ({}, {}) to intensity {}", grain.x, grain.y, intensity);
        grain.expose(intensity, 1.0);
    });

    for grain in &emulsion {
        if grain.activated {
            // println!("Grain at ({}, {}) activated", grain.x, grain.y);
        }
    }

    // save activated grains to output image
    tracing::info!("Saving activated grains to output image");
    let mut output = image::GrayImage::new(width, height);
    for grain in &emulsion {
        if grain.activated {
            let x = (grain.x * (width as usize)) / num_grains;
            let y = (grain.y * (height as usize)) / num_grains;
            output.put_pixel(x as u32, y as u32, image::Luma([u8::MAX]));
        }
    }
    output.save("output.png").unwrap();
}
