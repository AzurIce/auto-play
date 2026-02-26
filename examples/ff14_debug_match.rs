//! Debug template matching - take a screenshot and try to match templates
//!
//! cargo run --example ff14_debug_match --features windows

use auto_play::{AutoPlay, ControllerTrait, MatcherOptions, WindowsController};

const WINDOW_TITLE: &str = "最终幻想XIV";

fn main() -> anyhow::Result<()> {
    let controller = WindowsController::from_window_title(WINDOW_TITLE)?;
    let (w, h) = controller.screen_size();
    println!("Connected: {w}x{h}");

    let ap = AutoPlay::new(controller);

    // Take screenshot
    let screen = ap.screencap()?;
    screen.save("debug_screen.jpg")?;
    println!("Saved debug_screen.jpg");

    // Load templates
    let tpl_start = image::open("assets/start_crafting.png")?;
    let tpl_stop = image::open("assets/stop_crafting.png")?;
    println!(
        "Templates: start={}x{}, stop={}x{}",
        tpl_start.width(),
        tpl_start.height(),
        tpl_stop.width(),
        tpl_stop.height()
    );
    println!("Screen: {w}x{h}");

    let options = MatcherOptions::default();
    println!(
        "Match method: {:?}, threshold: {}",
        options.method, options.threshold
    );

    // Try matching start
    println!("\n--- Matching 'start_crafting' ---");
    let screen_luma = screen.to_luma32f();
    let tpl_start_luma = tpl_start.to_luma32f();
    let res = auto_play::cv::matcher::SingleMatcher::match_template(
        &screen_luma,
        &tpl_start_luma,
        &options,
    );
    let extremes = imageproc::template_matching::find_extremes(&res.matched_image);
    println!("Result: {:?}", res.result);
    println!(
        "Extremes: min={:.4} at {:?}, max={:.4} at {:?}",
        extremes.min_value,
        extremes.min_value_location,
        extremes.max_value,
        extremes.max_value_location
    );

    // Try matching stop
    println!("\n--- Matching 'stop_crafting' ---");
    let tpl_stop_luma = tpl_stop.to_luma32f();
    let res = auto_play::cv::matcher::SingleMatcher::match_template(
        &screen_luma,
        &tpl_stop_luma,
        &options,
    );
    let extremes = imageproc::template_matching::find_extremes(&res.matched_image);
    println!("Result: {:?}", res.result);
    println!(
        "Extremes: min={:.4} at {:?}, max={:.4} at {:?}",
        extremes.min_value,
        extremes.min_value_location,
        extremes.max_value,
        extremes.max_value_location
    );

    Ok(())
}
