//! Crop template images from FF14 reference screenshots for template matching.
//!
//! Run with: cargo run --example ff14_crop_templates --features windows
//!
//! This crops key UI elements from reference screenshots (a1.jpg, a2.jpg)
//! to create template images used for state detection during auto-crafting.

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let output_dir = Path::new("assets/ff14");
    std::fs::create_dir_all(output_dir)?;

    // === a1.jpg: Crafting Log (制作笔记) - 1345x955 ===
    let a1 = image::open("C:/Users/xiaob/Pictures/a1.jpg")?;
    println!("a1.jpg: {}x{}", a1.width(), a1.height());

    // Crop "开始制作作业" button (golden yellow button at bottom-right)
    // From the image, the button is at roughly:
    //   x: ~1080, y: ~905, width: ~260, height: ~45
    let btn_start = a1.crop_imm(1070, 895, 280, 55);
    btn_start.save(output_dir.join("btn_start_crafting.png"))?;
    println!(
        "Saved btn_start_crafting.png ({}x{})",
        btn_start.width(),
        btn_start.height()
    );

    // Also crop "开始简易制作作业" button for reference
    let btn_quick = a1.crop_imm(830, 895, 250, 55);
    btn_quick.save(output_dir.join("btn_quick_crafting.png"))?;
    println!(
        "Saved btn_quick_crafting.png ({}x{})",
        btn_quick.width(),
        btn_quick.height()
    );

    // Crop the title bar "制作笔记" to identify this window
    let title = a1.crop_imm(0, 0, 200, 40);
    title.save(output_dir.join("title_crafting_log.png"))?;
    println!(
        "Saved title_crafting_log.png ({}x{})",
        title.width(),
        title.height()
    );

    // === a2.jpg: Crafting Progress (制作进行中) - 711x598 ===
    let a2 = image::open("C:/Users/xiaob/Pictures/a2.jpg")?;
    println!("\na2.jpg: {}x{}", a2.width(), a2.height());

    // Crop the progress window title area with item name
    // The title "收藏用炼金溶剂" is at the top
    let progress_title = a2.crop_imm(0, 0, 400, 50);
    progress_title.save(output_dir.join("title_crafting_progress.png"))?;
    println!(
        "Saved title_crafting_progress.png ({}x{})",
        progress_title.width(),
        progress_title.height()
    );

    // Crop "作业中止" button (bottom-right, to detect crafting-in-progress state)
    let btn_abort = a2.crop_imm(530, 545, 170, 45);
    btn_abort.save(output_dir.join("btn_abort_crafting.png"))?;
    println!(
        "Saved btn_abort_crafting.png ({}x{})",
        btn_abort.width(),
        btn_abort.height()
    );

    // Crop the "进展" / "品质" labels area to detect crafting progress window
    let progress_area = a2.crop_imm(150, 90, 200, 40);
    progress_area.save(output_dir.join("label_progress.png"))?;
    println!(
        "Saved label_progress.png ({}x{})",
        progress_area.width(),
        progress_area.height()
    );

    // Crop the chevron icon (double arrow) at bottom center - unique to this window
    let chevron = a2.crop_imm(290, 545, 130, 45);
    chevron.save(output_dir.join("chevron_icon.png"))?;
    println!(
        "Saved chevron_icon.png ({}x{})",
        chevron.width(),
        chevron.height()
    );

    println!("\nAll templates saved to assets/ff14/");
    println!("Please verify the cropped images are correct before using them.");

    Ok(())
}
