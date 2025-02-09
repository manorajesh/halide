mod halide;
mod developer;
mod emulsion;

use developer::Developer;
use emulsion::Emulsion;
use halide::Halide;

use rayon::prelude::*;
use image::{ GrayImage, Luma };
use std::f32::consts::PI;

/// Generate a 2D Gaussian kernel with given sigma. Returns (kernel, size).
fn make_gaussian_kernel_2d(sigma: f32) -> (Vec<f32>, usize) {
    // We'll choose size to be roughly 3*sigma in each direction, min 3, ensure odd.
    let radius = (3.0 * sigma).ceil() as i32;
    let size = (2 * radius + 1).max(3) as usize;
    let mut kernel = vec![0.0; size * size];
    let mut sum = 0.0;
    let two_sigma2 = 2.0 * sigma * sigma;
    let half = (size / 2) as i32;

    // fill kernel with Gaussian
    for y in 0..size {
        for x in 0..size {
            let dx = (x as i32) - half;
            let dy = (y as i32) - half;
            let r2 = (dx * dx + dy * dy) as f32;
            let val = (-r2 / two_sigma2).exp();
            kernel[y * size + x] = val;
            sum += val;
        }
    }
    // normalize
    for v in kernel.iter_mut() {
        *v /= sum;
    }
    (kernel, size)
}

/// A generic 2D convolution for a single-channel f32 buffer.
fn convolve_2d(
    width: usize,
    height: usize,
    input: &[f32],
    kernel: &[f32],
    k_size: usize
) -> Vec<f32> {
    let mut output = vec![0.0; width * height];
    let half_k = (k_size / 2) as i32;

    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            for ky in 0..k_size {
                for kx in 0..k_size {
                    let dx = (kx as i32) - half_k;
                    let dy = (ky as i32) - half_k;
                    let nx = (x as i32) + dx;
                    let ny = (y as i32) + dy;
                    if nx >= 0 && nx < (width as i32) && ny >= 0 && ny < (height as i32) {
                        let idx_in = (ny as usize) * width + (nx as usize);
                        let idx_k = ky * k_size + kx;
                        sum += input[idx_in] * kernel[idx_k];
                    }
                }
            }
            let idx_out = y * width + x;
            output[idx_out] = sum;
        }
    }
    output
}

/// Two-pass halation simulation:
/// 1) Convolve input_exposure "downward" with a small kernel (sigma_down).
/// 2) Multiply by reflection_factor (0..1).
/// 3) Convolve that result "upward" with another kernel (sigma_up).
/// 4) Add it back to the original exposure to get the final halation result.
fn simulate_halation_2d(
    width: usize,
    height: usize,
    input_exposure: &[f32],
    reflection_factor: f32,
    sigma_down: f32,
    sigma_up: f32
) -> Vec<f32> {
    // Make downward and upward kernels
    let (kernel_down, kd_size) = make_gaussian_kernel_2d(sigma_down);
    let (kernel_up, ku_size) = make_gaussian_kernel_2d(sigma_up);

    // 1) downward scatter
    let transmitted = convolve_2d(width, height, input_exposure, &kernel_down, kd_size);

    // 2) reflection at base
    let reflected: Vec<f32> = transmitted
        .iter()
        .map(|&val| val * reflection_factor)
        .collect();

    // 3) upward scatter
    let upward = convolve_2d(width, height, &reflected, &kernel_up, ku_size);

    // 4) final halation = original + upward
    let mut final_exposure = vec![0.0; width * height];
    for i in 0..final_exposure.len() {
        final_exposure[i] = input_exposure[i] + upward[i];
    }

    final_exposure
}

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Creating emulsion");

    // 1) Load the input image
    let image = image::open("test_images/inputs/input.jpeg").unwrap();
    let image = image.to_luma16();
    let (width, height) = image.dimensions();
    let (w_usize, h_usize) = (width as usize, height as usize);

    // 2) Convert to a float exposure buffer [0..1]
    let mut direct_exposure = vec![0.0; (w_usize * h_usize)];
    for y in 0..h_usize {
        for x in 0..w_usize {
            let pixel_val = image.get_pixel(x as u32, y as u32).0[0];
            direct_exposure[y * w_usize + x] = (pixel_val as f32) / (u16::MAX as f32);
        }
    }

    // 3) Halation pass
    tracing::info!("Applying halation pass...");
    let reflection_factor = 0.6; // fraction of light reflecting off the base
    let sigma_down = 3.0; // how wide the scatter downward
    let sigma_up = 5.0; // how wide the scatter upward
    let halated_exposure = simulate_halation_2d(
        w_usize,
        h_usize,
        &direct_exposure,
        reflection_factor,
        sigma_down,
        sigma_up
    );

    // 4) Build the "emulsion" array of Halides from the halated exposures
    //    (One Halide per pixel, as in your current approach)
    let mut emulsion = Vec::with_capacity(w_usize * h_usize);
    for y in 0..h_usize {
        for x in 0..w_usize {
            let intensity = halated_exposure[y * w_usize + x];
            let mut halide = Halide::from_pixel(x, y, intensity, 1000.0);

            let subhalides: Vec<Halide> = (0..5)
                .into_iter()
                .map(|_| { Halide::from_pixel(x, y, intensity, 1000.0) })
                .collect();

            for sub in subhalides {
                halide.average_with_halide(&sub);
            }

            emulsion.push(halide);
        }
    }

    // 5) Develop emulsion
    tracing::info!("Developing emulsion");
    let dev = Developer {
        strength: 0.1,
        max_development: 1.0,
    };
    let dt = 0.1;
    for grain in emulsion.iter_mut() {
        Halide::develop_grain(grain, &dev, dt);
    }

    // 6) Convert final vector of Halides to Emulsion and render
    tracing::info!("Saving activated grains to output image");
    let emulsion = Emulsion::from(emulsion);
    let output = emulsion.render_emulsion(width, height);

    // 7) Save the final image
    output.save("test_images/negative.exr").unwrap();
    tracing::info!("Done!");
}
