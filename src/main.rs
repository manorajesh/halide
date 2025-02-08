mod halide;
mod developer;
mod emulsion;

use developer::Developer;
use emulsion::Emulsion;
use halide::Halide;

use rayon::prelude::*;

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Creating emulsion");

    // open input image
    let image = image::open("test_images/inputs/input.png").unwrap();
    let image = image.to_luma16();
    let (width, height) = image.dimensions();

    let num_grains = 10_000_000;
    let mut emulsion = Emulsion::create_random_emulsion(width, height, num_grains);

    // expose emulsion to image
    tracing::info!("Exposing emulsion to image");
    emulsion.grains.par_iter_mut().for_each(|grain| {
        let pixel_val = image.get_pixel(grain.x as u32, grain.y as u32).0[0];
        let intensity = (pixel_val as f32) / (u16::MAX as f32);
        grain.expose(intensity, 700.0);
    });

    // develop emulsion
    tracing::info!("Developing emulsion");
    let dev = Developer {
        strength: 0.1,
        max_development: 1.0,
    };
    let dt = 0.1;
    emulsion.grains.par_iter_mut().for_each(|grain| {
        Halide::develop_grain(grain, &dev, dt);
    });

    // save activated grains to output image
    tracing::info!("Saving activated grains to negative image");
    let output = emulsion.render_emulsion(width, height);
    output.save("test_images/negative.png").unwrap();
}
