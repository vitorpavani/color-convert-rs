use color_convert_rs::batch;

#[test]
fn stride_3_rgb_matches_stride_4_rgba() {
    let rgb_data: Vec<u8> = (0..30).collect();
    let mut rgba_data = Vec::with_capacity(40);
    for chunk in rgb_data.chunks_exact(3) {
        rgba_data.extend_from_slice(chunk);
        rgba_data.push(255);
    }

    let lab_rgb = batch::rgb_to_lab(&rgb_data, 3);
    let lab_rgba = batch::rgb_to_lab(&rgba_data, 4);

    assert_eq!(lab_rgb.len(), lab_rgba.len());
    for (a, b) in lab_rgb.iter().zip(lab_rgba.iter()) {
        assert_eq!(
            a, b,
            "RGB and RGBA stride must produce identical LAB output"
        );
    }
}

#[test]
fn stride_3_xyz_matches_stride_4() {
    let rgb_data: Vec<u8> = (0..30).collect();
    let mut rgba_data = Vec::with_capacity(40);
    for chunk in rgb_data.chunks_exact(3) {
        rgba_data.extend_from_slice(chunk);
        rgba_data.push(128);
    }

    let xyz_rgb = batch::rgb_to_xyz(&rgb_data, 3);
    let xyz_rgba = batch::rgb_to_xyz(&rgba_data, 4);

    assert_eq!(xyz_rgb.len(), xyz_rgba.len());
    for (a, b) in xyz_rgb.iter().zip(xyz_rgba.iter()) {
        assert_eq!(a, b);
    }
}

#[test]
fn batch_matches_typed_simd() {
    use color_convert_rs::simd;
    let pixels: Vec<[u8; 3]> = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
    let flat: Vec<u8> = pixels.iter().flat_map(|p| p.iter().copied()).collect();

    let typed = simd::rgb_to_lab_batch(&pixels);
    let flat_result = batch::rgb_to_lab(&flat, 3);

    assert_eq!(typed, flat_result);
}

#[test]
fn cmyk_stride_works() {
    let rgb_data: Vec<u8> = vec![255, 0, 0, 0, 255, 0];
    let cmyk = batch::rgb_to_cmyk(&rgb_data, 3);
    assert_eq!(cmyk.len(), 2);
    assert_eq!(cmyk[0].len(), 4);
}

#[test]
#[should_panic(expected = "stride must be at least 3")]
fn stride_too_small_panics() {
    let data = vec![1u8, 2];
    batch::rgb_to_lab(&data, 2);
}

#[cfg(feature = "image")]
#[test]
fn image_dynamic_image_to_lab() {
    let img = image::DynamicImage::new_rgb8(2, 2);
    let lab = color_convert_rs::batch::image::to_lab(&img);
    assert_eq!(lab.len(), 4, "2x2 image = 4 pixels");
    for px in &lab {
        assert_eq!(px.len(), 3);
    }
}
