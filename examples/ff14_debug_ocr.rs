//! Debug OCR on FF14 crafting progress window
//!
//! Tests both plain OCR and enhanced (upscale+binarize) OCR on crafting numbers.
//! Uses "作業中止" template match as anchor to locate progress/quality values.
//!
//! cargo run --example ff14_debug_ocr --features windows

use auto_play::controller::windows::ocr;
use auto_play::controller::windows::WindowsController;
use auto_play::controller::ControllerTrait;
use auto_play::cv::matcher::{MatcherOptions, SingleMatcher};
use std::thread;
use std::time::Duration;

/// Offsets from the "作業中止" button center to the progress/quality value regions.
/// These are approximate pixel offsets based on the 3442x1396 screenshot.
/// anchor = center of 作業中止 button (~1028, 1020)
/// 进展 value: roughly at (1063, 631) → offset from anchor: (+35, -389)
/// 品質 value: roughly at (1063, 691) → offset from anchor: (+35, -329)
const PROGRESS_OFFSET: (i32, i32) = (35, -389);
const QUALITY_OFFSET: (i32, i32) = (35, -329);
/// Size of the crop region around each value
const VALUE_CROP_SIZE: (u32, u32) = (250, 50);

fn main() -> anyhow::Result<()> {
    let controller = WindowsController::from_window_title("最终幻想XIV")?;
    let (w, h) = controller.screen_size();
    println!("Connected: {w}x{h}");

    thread::sleep(Duration::from_millis(300));

    let screen = controller.screencap()?;
    println!("Screenshot: {}x{}", screen.width(), screen.height());

    // Step 1: Find "作業中止" as anchor
    let tpl_stop = image::open("assets/stop_crafting.png")?;
    let opts = MatcherOptions::default();
    let screen_luma = screen.to_luma32f();
    let tpl_luma = tpl_stop.to_luma32f();
    let res = SingleMatcher::match_template(&screen_luma, &tpl_luma, &opts);

    let Some(m) = res.result else {
        println!("'作業中止' not found — make sure crafting is in progress!");
        // Fallback: full screen OCR
        println!("\n=== Full Screen OCR (fallback) ===");
        let result = ocr::ocr_from_image(&screen)?;
        for block in &result.blocks {
            println!(
                "  [{:.0},{:.0} {:.0}x{:.0}] {}",
                block.x, block.y, block.width, block.height, block.text
            );
        }
        return Ok(());
    };

    let anchor_x = m.rect.x + m.rect.width / 2;
    let anchor_y = m.rect.y + m.rect.height / 2;
    println!(
        "Anchor '作業中止' at ({}, {}), score={:.4}",
        anchor_x, anchor_y, m.value
    );

    // Step 2: Crop regions for progress and quality
    let regions = [
        ("进展 (Progress)", PROGRESS_OFFSET),
        ("品質 (Quality)", QUALITY_OFFSET),
    ];

    for (label, (dx, dy)) in &regions {
        let cx = (anchor_x as i32 + dx).max(0) as u32;
        let cy = (anchor_y as i32 + dy).max(0) as u32;
        let crop_x = cx.saturating_sub(VALUE_CROP_SIZE.0 / 2);
        let crop_y = cy.saturating_sub(VALUE_CROP_SIZE.1 / 2);
        let crop_w = VALUE_CROP_SIZE.0.min(screen.width() - crop_x);
        let crop_h = VALUE_CROP_SIZE.1.min(screen.height() - crop_y);

        println!("\n=== {label} ===");
        println!("  Crop: ({crop_x}, {crop_y}) {crop_w}x{crop_h}");

        // Plain OCR on cropped region
        let plain = ocr::ocr_region(&screen, crop_x, crop_y, crop_w, crop_h)?;
        println!("  Plain OCR: {:?}", plain.text.trim());

        // Enhanced OCR: 3x upscale + binarize (threshold 128)
        let enhanced = ocr::ocr_region_enhanced(&screen, crop_x, crop_y, crop_w, crop_h, 3, 128)?;
        println!("  Enhanced OCR (3x, t=128): {:?}", enhanced.text.trim());

        // Enhanced OCR: 4x upscale + binarize (threshold 100)
        let enhanced2 = ocr::ocr_region_enhanced(&screen, crop_x, crop_y, crop_w, crop_h, 4, 100)?;
        println!("  Enhanced OCR (4x, t=100): {:?}", enhanced2.text.trim());

        // Try parsing
        for result in [&plain, &enhanced, &enhanced2] {
            if let Some((cur, max)) = ocr::parse_fraction(result.text.trim()) {
                println!("  -> Parsed: {cur}/{max}");
            }
        }

        // Save cropped images for visual inspection
        let cropped = screen.crop_imm(crop_x, crop_y, crop_w, crop_h);
        let filename = format!("debug_ocr_{}.png", label.split(' ').next().unwrap());
        cropped.save(&filename)?;
        println!("  Saved: {filename}");
    }

    Ok(())
}
