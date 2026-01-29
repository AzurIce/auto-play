pub use ap_adb as adb;
pub use ap_controller as controller;
pub use ap_cv as cv;

pub mod action;
pub mod nav;

// Re-export the Controller trait and concrete implementations
pub use controller::{AndroidController, Controller, ControllerTrait};

#[cfg(feature = "windows")]
pub use controller::WindowsController;

// Re-export specific items users might need frequently
pub use adb::Device;
pub use image::DynamicImage;

// Export CV related options for matching
pub use cv::core::template_matching::MatchTemplateMethod;
pub use cv::matcher::MatcherOptions;

use cv::matcher::SingleMatcher;
use std::any::Any;
use std::time::Duration;

/// The main entry point for automation tasks.
///
/// `AutoPlay` integrates device control (via `ap-controller`) and computer vision (via `ap-cv`)
/// to provide a high-level API for game automation scripts.
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
pub struct AutoPlay {
    controller: Controller,
}

impl AutoPlay {
    pub fn new<T: ControllerTrait + Any + Send + 'static>(controller: T) -> Self {
        Self {
            controller: Controller::new(controller),
        }
    }

    pub fn controller(&self) -> &Controller {
        &self.controller
    }

    pub fn controller_ref<T: ControllerTrait + 'static>(&self) -> Option<&T> {
        self.controller.downcast_ref::<T>()
    }

    pub fn screen_size(&self) -> (u32, u32) {
        self.controller.screen_size()
    }

    pub fn scale_factor(&self) -> f32 {
        self.controller.scale_factor()
    }

    pub fn screencap(&self) -> anyhow::Result<DynamicImage> {
        self.controller.screencap()
    }

    pub fn screencap_scaled(&self) -> anyhow::Result<DynamicImage> {
        self.controller.screencap()
    }

    pub fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.controller.click(x, y)
    }

    pub fn click_scaled(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.controller.click(x, y)
    }

    pub fn swipe(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        self.controller
            .swipe(start, end, duration, slope_in, slope_out)
    }

    pub fn swipe_scaled(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        self.controller
            .swipe(start, end, duration, slope_in, slope_out)
    }

    pub fn find_image(
        &self,
        template: &DynamicImage,
        options: &MatcherOptions,
    ) -> anyhow::Result<Option<image::math::Rect>> {
        let screen = self.screencap()?;
        let screen_luma = screen.to_luma32f();
        let template_luma = template.to_luma32f();
        let res = SingleMatcher::match_template(&screen_luma, &template_luma, options);
        Ok(res.result.map(|m| m.rect))
    }

    pub fn find_image_default(
        &self,
        template: &DynamicImage,
    ) -> anyhow::Result<Option<image::math::Rect>> {
        self.find_image(template, &MatcherOptions::default())
    }

    pub fn click_image(
        &self,
        template: &DynamicImage,
        options: &MatcherOptions,
    ) -> anyhow::Result<bool> {
        if let Some(rect) = self.find_image(template, options)? {
            self.controller
                .click(rect.x + rect.width / 2, rect.y + rect.height / 2)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

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
