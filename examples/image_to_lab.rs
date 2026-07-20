//! Load a JPEG/PNG, convert all pixels to LAB via SIMD, measure throughput.
//!
//! Run: `cargo run --release --example image_to_lab --features image -- your_photo.jpg`

use std::time::Instant;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/sample.jpg".to_string());

    let img = image::open(&path).unwrap_or_else(|e| {
        eprintln!("Failed to open {path}: {e}");
        eprintln!(
            "Usage: cargo run --release --example image_to_lab --features image -- <image.jpg>"
        );
        eprintln!("Generating a synthetic 1920×1080 image instead.");
        generate_synthetic()
    });

    let (w, h) = (img.width(), img.height());
    let n_pixels = (w * h) as usize;
    println!("Loaded {path}: {w}×{h} = {n_pixels} pixels");

    let rgb = img.to_rgb8();
    let raw: &[u8] = rgb.as_raw();

    let routes = [
        (
            "rgb→lab",
            color_convert_rs::batch::rgb_to_lab as fn(&[u8], usize) -> Vec<[f32; 3]>,
        ),
        ("rgb→xyz", color_convert_rs::batch::rgb_to_xyz),
        ("rgb→oklab", color_convert_rs::batch::rgb_to_oklab),
        ("rgb→hsl", color_convert_rs::batch::rgb_to_hsl),
        ("rgb→hsv", color_convert_rs::batch::rgb_to_hsv),
    ];

    println!("\nRoute       | Time      | Throughput");
    println!("------------ | --------- | ----------");

    for (name, f) in &routes {
        let start = Instant::now();
        let result = f(raw, 3);
        let elapsed = start.elapsed();

        let mpx_per_sec = (n_pixels as f64) / elapsed.as_secs_f64() / 1_000_000.0;
        println!(
            " {:10} | {:>6.2}ms | {:.1} M px/s (first pixel: [{:.1}, {:.1}, {:.1}])",
            name,
            elapsed.as_secs_f64() * 1000.0,
            mpx_per_sec,
            result[0][0],
            result[0][1],
            result[0][2]
        );
    }

    let cmyk_start = Instant::now();
    let cmyk = color_convert_rs::batch::rgb_to_cmyk(raw, 3);
    let cmyk_elapsed = cmyk_start.elapsed();
    let cmyk_mpx = (n_pixels as f64) / cmyk_elapsed.as_secs_f64() / 1_000_000.0;
    println!(
        " {:10} | {:>6.2}ms | {:.1} M px/s",
        "rgb→cmyk",
        cmyk_elapsed.as_secs_f64() * 1000.0,
        cmyk_mpx
    );
    let _ = cmyk;
}

fn generate_synthetic() -> image::DynamicImage {
    let mut img = image::DynamicImage::new_rgb8(1920, 1080);
    let buf = img.as_mut_rgb8().unwrap();
    for (i, px) in buf.pixels_mut().enumerate() {
        let r = (i % 256) as u8;
        let g = ((i / 256) % 256) as u8;
        let b = ((i / 65536) % 256) as u8;
        *px = image::Rgb([r, g, b]);
    }
    img
}
