//! Stride-aware batch color conversion for raw pixel buffers.
//!
//! The [`crate::simd`] functions take `&[[u8; 3]]` (typed RGB triplets). Real
//! image data arrives as flat `&[u8]` — interleaved RGB (`[r,g,b,r,g,b,...]`)
//! or RGBA (`[r,g,b,a,r,g,b,a,...]`). This module bridges that gap so any
//! pixel buffer can be converted without copying or re-packing.
//!
//! ## Quick start
//!
//! ```no_run
//! # #[cfg(feature = "image")] {
//! let img = image::open("photo.jpg").unwrap();
//! let rgb = img.to_rgb8();
//! let lab = color_convert_rs::batch::rgb_to_lab(&rgb, 3); // 3 = RGB stride
//! println!("{} pixels → LAB", lab.len());
//! # }
//! ```
//!
//! ## ImageData (browser)
//!
//! ```ignore
//! let imageData = ctx.getImageData(0, 0, w, h);
//! let lab = color_convert_rs::batch::rgb_to_lab(imageData.data, 4); // 4 = RGBA
//! ```
//!
//! ## Stride
//!
//! `stride` = bytes per pixel. Use `3` for RGB, `4` for RGBA. Channels beyond
//! the third are ignored (alpha is dropped).

use crate::simd;
use crate::simd_cmyk;
use crate::simd_hsl;
use crate::simd_hsv;
use crate::simd_oklab;

/// Extract RGB triplets from a flat byte buffer with the given stride.
fn extract_rgb(input: &[u8], stride: usize) -> Vec<[u8; 3]> {
    assert!(stride >= 3, "stride must be at least 3 (RGB), got {stride}");
    input
        .chunks_exact(stride)
        .map(|c| [c[0], c[1], c[2]])
        .collect()
}

macro_rules! rgb_batch_fn {
    ($name:ident, $simd:path, $out_chans:expr) => {
        #[doc = concat!("Convert flat RGB/RGBA bytes to the target space. `stride` = bytes per pixel (3 for RGB, 4 for RGBA).")]
        pub fn $name(input: &[u8], stride: usize) -> Vec<[f32; $out_chans]> {
            let pixels = extract_rgb(input, stride);
            $simd(&pixels)
        }
    };
}

rgb_batch_fn!(rgb_to_lab, simd::rgb_to_lab_batch, 3);
rgb_batch_fn!(rgb_to_xyz, simd::rgb_to_xyz_batch, 3);
rgb_batch_fn!(rgb_to_hsl, simd_hsl::rgb_to_hsl_batch, 3);
rgb_batch_fn!(rgb_to_hsv, simd_hsv::rgb_to_hsv_batch, 3);
rgb_batch_fn!(rgb_to_oklab, simd_oklab::rgb_to_oklab_batch, 3);
rgb_batch_fn!(rgb_to_cmyk, simd_cmyk::rgb_to_cmyk_batch, 4);

#[cfg(feature = "image")]
pub mod image {
    //! Convenience wrappers for the [`image`](https://docs.rs/image) crate.
    //!
    //! Enable with `--features image`. Accepts `DynamicImage`, `ImageBuffer`,
    //! and `RgbImage` directly — no manual byte extraction needed.

    use crate::batch;

    /// Convert an [`image::DynamicImage`] to LAB pixels (one per source pixel).
    pub fn to_lab(img: &image::DynamicImage) -> Vec<[f32; 3]> {
        let rgb = img.to_rgb8();
        batch::rgb_to_lab(rgb.as_raw(), 3)
    }

    /// Convert an [`image::DynamicImage`] to XYZ pixels.
    pub fn to_xyz(img: &image::DynamicImage) -> Vec<[f32; 3]> {
        let rgb = img.to_rgb8();
        batch::rgb_to_xyz(rgb.as_raw(), 3)
    }

    /// Convert an [`image::DynamicImage`] to OkLab pixels.
    pub fn to_oklab(img: &image::DynamicImage) -> Vec<[f32; 3]> {
        let rgb = img.to_rgb8();
        batch::rgb_to_oklab(rgb.as_raw(), 3)
    }

    /// Convert an [`image::DynamicImage`] to HSL pixels.
    pub fn to_hsl(img: &image::DynamicImage) -> Vec<[f32; 3]> {
        let rgb = img.to_rgb8();
        batch::rgb_to_hsl(rgb.as_raw(), 3)
    }

    /// Convert an [`image::DynamicImage`] to HSV pixels.
    pub fn to_hsv(img: &image::DynamicImage) -> Vec<[f32; 3]> {
        let rgb = img.to_rgb8();
        batch::rgb_to_hsv(rgb.as_raw(), 3)
    }

    /// Convert an [`image::DynamicImage`] to CMYK pixels.
    pub fn to_cmyk(img: &image::DynamicImage) -> Vec<[f32; 4]> {
        let rgb = img.to_rgb8();
        batch::rgb_to_cmyk(rgb.as_raw(), 3)
    }
}
