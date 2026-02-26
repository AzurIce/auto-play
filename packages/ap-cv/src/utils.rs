use std::path::Path;

use image::{ImageBuffer, Luma};

pub fn normalize_luma32f(
    image: &ImageBuffer<Luma<f32>, Vec<f32>>,
) -> ImageBuffer<Luma<f32>, Vec<f32>> {
    let max = image
        .as_raw()
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    let min = image
        .as_raw()
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
    ImageBuffer::from_vec(
        image.width(),
        image.height(),
        image
            .as_raw()
            .into_iter()
            .map(|x| (x - min) / (max - min))
            .collect(),
    )
    .unwrap()
}

pub fn luma32f_to_luma8(
    image: &ImageBuffer<Luma<f32>, Vec<f32>>,
) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    ImageBuffer::from_vec(
        image.width(),
        image.height(),
        image.as_raw().iter().map(|x| (x * 255.0) as u8).collect(),
    )
    .unwrap()
}

/// save `image` to `path`,
/// `normalize` indicated whether a linear min-max normalize should be performed
pub fn save_luma32f<P: AsRef<Path>>(
    image: &ImageBuffer<Luma<f32>, Vec<f32>>,
    path: P,
    normalize: bool,
) {
    let image = if normalize {
        normalize_luma32f(image)
    } else {
        image.clone()
    };
    let res_image = luma32f_to_luma8(&image);
    res_image.save(path).unwrap();
}
