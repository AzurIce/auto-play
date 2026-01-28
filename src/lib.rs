pub use ap_adb as adb;
pub use ap_controller as controller;
pub use ap_cv as cv;

// Re-export the Controller trait and concrete implementations
pub use controller::Controller;
pub use controller::AndroidController;

#[cfg(feature = "windows")]
pub use controller::WindowsController;

// Re-export specific items users might need frequently
pub use adb::Device;
pub use image::DynamicImage;

// Export CV related options for matching
pub use cv::core::template_matching::MatchTemplateMethod;
pub use cv::matcher::MatcherOptions;

use cv::matcher::SingleMatcher;
use std::time::Duration;

/// The main entry point for automation tasks.
///
/// `AutoPlay` integrates device control (via `ap-controller`) and computer vision (via `ap-cv`)
/// to provide a high-level API for game automation scripts.
///
/// # Type Parameter
/// - `T`: Any type that implements the [`Controller`] trait (e.g., `AndroidController`, `WindowsController`)
///
/// # Example
/// ```ignore
/// // For Android
/// let controller = AndroidController::connect("192.168.1.3:40919")?;
/// let auto_play = AutoPlay::new(controller);
///
/// // For Windows
/// let controller = WindowsController::from_window_title("Game Window")?;
/// let auto_play = AutoPlay::new(controller);
///
/// // Same API for both platforms
/// auto_play.click_image(&template, &MatcherOptions::default())?;
/// ```
pub struct AutoPlay<T: Controller> {
    controller: T,
}

impl<T: Controller> AutoPlay<T> {
    pub fn new(controller: T) -> Self {
        Self { controller }
    }

    /// Access the underlying controller for low-level operations.
    pub fn controller(&self) -> &T {
        &self.controller
    }

    /// Access the underlying controller mutably for low-level operations.
    pub fn controller_mut(&mut self) -> &mut T {
        &mut self.controller
    }

    /// Get the screen size from the controller.
    pub fn screen_size(&self) -> (u32, u32) {
        self.controller.screen_size()
    }

    /// Get the scale factor from the controller.
    pub fn scale_factor(&self) -> f32 {
        self.controller.scale_factor()
    }

    /// Take a screenshot.
    pub fn screencap(&self) -> anyhow::Result<DynamicImage> {
        self.controller.screencap()
    }

    /// Take a scaled screenshot (1080p).
    pub fn screencap_scaled(&self) -> anyhow::Result<DynamicImage> {
        self.controller.screencap_scaled()
    }

    /// Click at the specified coordinates.
    pub fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.controller.click(x, y)
    }

    /// Click at coordinates scaled from 1920x1080.
    pub fn click_scaled(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.controller.click_scaled(x, y)
    }

    /// Perform a swipe gesture.
    pub fn swipe(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        self.controller.swipe(start, end, duration, slope_in, slope_out)
    }

    /// Perform a swipe with coordinates scaled from 1920x1080.
    pub fn swipe_scaled(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        self.controller.swipe_scaled(start, end, duration, slope_in, slope_out)
    }

    // ===== Computer Vision Methods =====

    /// Searches for a template image on the current screen with custom options.
    ///
    /// Returns the bounding rectangle of the match if found, or `None`.
    pub fn find_image(
        &self,
        template: &DynamicImage,
        options: &MatcherOptions,
    ) -> anyhow::Result<Option<image::math::Rect>> {
        let screen = self.controller.screencap()?;

        let screen_luma = screen.to_luma32f();
        let template_luma = template.to_luma32f();

        let res = SingleMatcher::match_template(&screen_luma, &template_luma, options);

        Ok(res.result.map(|m| m.rect))
    }

    /// A shortcut for [`Self::find_image`] using default options.
    pub fn find_image_default(
        &self,
        template: &DynamicImage,
    ) -> anyhow::Result<Option<image::math::Rect>> {
        self.find_image(template, &MatcherOptions::default())
    }

    /// Searches for a template image and clicks it if found.
    pub fn click_image(
        &self,
        template: &DynamicImage,
        options: &MatcherOptions,
    ) -> anyhow::Result<bool> {
        if let Some(rect) = self.find_image(template, options)? {
            self.controller.click_in_rect(rect)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Repeatedly attempts to find and click an image for a specified duration.
    pub fn wait_and_click_image(
        &self,
        template: &DynamicImage,
        options: &MatcherOptions,
        timeout: Duration,
    ) -> anyhow::Result<bool> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if self.click_image(template, options)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Wait for a template image to appear on screen.
    ///
    /// Returns the bounding rectangle if found within timeout, or `None`.
    pub fn wait_for_image(
        &self,
        template: &DynamicImage,
        options: &MatcherOptions,
        timeout: Duration,
    ) -> anyhow::Result<Option<image::math::Rect>> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if let Some(rect) = self.find_image(template, options)? {
                return Ok(Some(rect));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
