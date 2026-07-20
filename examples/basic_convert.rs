//! Basic single-color conversion: rgb → lab via the public `convert` API.
//!
//! Run: `cargo run --example basic_convert`

use color_convert_rs::{Color, Model, convert_rounded};

fn main() {
    let orange = Color::Rgb([255.0, 128.0, 0.0]);

    let lab = convert_rounded(Model::Rgb, Model::Lab, orange.clone()).unwrap();
    let hsl = convert_rounded(Model::Rgb, Model::Hsl, orange.clone()).unwrap();
    let cmyk = convert_rounded(Model::Rgb, Model::Cmyk, orange.clone()).unwrap();
    let hex = convert_rounded(Model::Rgb, Model::Hex, orange.clone()).unwrap();
    let keyword = convert_rounded(Model::Rgb, Model::Keyword, orange).unwrap();

    println!("rgb(255, 128, 0) conversions:");
    println!("  lab:     {:?}", lab);
    println!("  hsl:     {:?}", hsl);
    println!("  cmyk:    {:?}", cmyk);
    println!("  hex:     {:?}", hex);
    println!("  keyword: {:?}", keyword);
}
