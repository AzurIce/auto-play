//! Windows OCR using Windows.Media.Ocr.OcrEngine API
//!
//! Provides OCR functionality for screenshots without writing to disk.

use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;

/// A single recognized text region with its bounding box
#[derive(Debug, Clone)]
pub struct OcrTextBlock {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Full OCR result from a screenshot
#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub blocks: Vec<OcrTextBlock>,
}

/// Perform OCR on an RGBA image buffer.
///
/// Takes raw RGBA pixel data and image dimensions, returns recognized text with positions.
pub fn ocr_from_rgba(rgba_data: &[u8], width: u32, height: u32) -> anyhow::Result<OcrResult> {
    // Windows OCR expects BGRA8 format, convert from RGBA
    let mut bgra_data = rgba_data.to_vec();
    for pixel in bgra_data.chunks_exact_mut(4) {
        pixel.swap(0, 2); // R <-> B
    }

    // Create SoftwareBitmap from pixel data
    let bitmap = SoftwareBitmap::Create(BitmapPixelFormat::Bgra8, width as i32, height as i32)?;

    // Copy pixel data into the bitmap
    {
        use windows::Win32::System::WinRT::IMemoryBufferByteAccess;

        let buffer =
            bitmap.LockBuffer(windows::Graphics::Imaging::BitmapBufferAccessMode::Write)?;
        let reference = buffer.CreateReference()?;
        let byte_access: IMemoryBufferByteAccess = windows::core::Interface::cast(&reference)?;
        unsafe {
            let mut ptr = std::ptr::null_mut();
            let mut len = 0u32;
            byte_access.GetBuffer(&mut ptr, &mut len)?;
            let dest = std::slice::from_raw_parts_mut(ptr, len as usize);
            // The bitmap buffer may have padding per row
            let stride = len as usize / height as usize;
            let src_stride = (width * 4) as usize;
            for row in 0..height as usize {
                let src_start = row * src_stride;
                let dst_start = row * stride;
                dest[dst_start..dst_start + src_stride]
                    .copy_from_slice(&bgra_data[src_start..src_start + src_stride]);
            }
        }
    }

    // Create OCR engine and recognize
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
    let result = engine.RecognizeAsync(&bitmap)?.get()?;

    let mut full_text = String::new();
    let mut blocks = Vec::new();

    for line in result.Lines()? {
        let text = line.Text()?.to_string();
        full_text.push_str(&text);
        full_text.push('\n');

        // Calculate line bounding rect from words
        let words = line.Words()?;
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for word in &words {
            let r = word.BoundingRect()?;
            min_x = min_x.min(r.X);
            min_y = min_y.min(r.Y);
            max_x = max_x.max(r.X + r.Width);
            max_y = max_y.max(r.Y + r.Height);
        }

        if min_x < f32::MAX {
            blocks.push(OcrTextBlock {
                text,
                x: min_x,
                y: min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            });
        }
    }

    Ok(OcrResult {
        text: full_text,
        blocks,
    })
}

/// Perform OCR on a DynamicImage directly.
pub fn ocr_from_image(image: &image::DynamicImage) -> anyhow::Result<OcrResult> {
    let rgba = image.to_rgba8();
    ocr_from_rgba(rgba.as_raw(), image.width(), image.height())
}

/// Perform OCR on a cropped region of an image.
pub fn ocr_region(
    image: &image::DynamicImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> anyhow::Result<OcrResult> {
    let cropped = image.crop_imm(x, y, width, height);
    ocr_from_image(&cropped)
}

/// Enhanced OCR: crop region → upscale → binarize → OCR.
///
/// This significantly improves accuracy for small game fonts by:
/// 1. Cropping a tight region around the text
/// 2. Upscaling 3x with Lanczos3 for sharper edges
/// 3. Converting to high-contrast black-on-white via threshold
pub fn ocr_region_enhanced(
    image: &image::DynamicImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    scale: u32,
    threshold: u8,
) -> anyhow::Result<OcrResult> {
    let cropped = image.crop_imm(x, y, width, height);

    // Upscale
    let new_w = cropped.width() * scale;
    let new_h = cropped.height() * scale;
    let upscaled = cropped.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

    // Binarize: grayscale → threshold → black text on white background
    let gray = upscaled.to_luma8();
    let mut rgba = image::RgbaImage::new(new_w, new_h);
    for (x, y, luma) in gray.enumerate_pixels() {
        let val = if luma.0[0] > threshold { 255u8 } else { 0u8 };
        rgba.put_pixel(x, y, image::Rgba([val, val, val, 255]));
    }

    let processed = image::DynamicImage::ImageRgba8(rgba);
    ocr_from_image(&processed)
}

/// Parse a "current/max" string (e.g. "0/340") into (current, max).
pub fn parse_fraction(text: &str) -> Option<(u32, u32)> {
    // Clean up common OCR artifacts and normalize
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '/')
        .collect();
    let parts: Vec<&str> = cleaned.split('/').collect();
    if parts.len() == 2 {
        let current = parts[0].parse::<u32>().ok()?;
        let max = parts[1].parse::<u32>().ok()?;
        Some((current, max))
    } else {
        None
    }
}
