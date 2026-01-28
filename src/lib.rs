pub use ap_adb as adb;
pub use ap_controller as controller;
use ap_controller::android::Controller;
pub use ap_cv as cv;

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
pub struct AutoPlay {
    controller: Controller,
}

impl AutoPlay {
    /// Connects to a device via ADB serial and initializes the automation runtime.
    pub fn connect(serial: impl AsRef<str>) -> anyhow::Result<Self> {
        let device = adb::connect(serial)?;
        Self::from_device(device)
    }

    /// Initializes `AutoPlay` from an existing ADB device connection.
    pub fn from_device(device: Device) -> anyhow::Result<Self> {
        let controller = Controller::from_device(device)?;
        Ok(Self { controller })
    }

    /// Access the underlying [`Controller`] for low-level operations.
    pub fn controller(&self) -> &Controller {
        &self.controller
    }

    /// Captures the current screen content.
    pub fn screencap(&self) -> anyhow::Result<DynamicImage> {
        self.controller.screencap()
    }

    /// Checks if the device screen is currently on.
    pub fn is_screen_on(&self) -> anyhow::Result<bool> {
        self.controller.is_screen_on()
    }

    /// Wakes up the device if the screen is off.
    pub fn ensure_screen_on(&self) -> anyhow::Result<()> {
        self.controller.ensure_screen_on()
    }

    /// Gets the device ABI (e.g., arm64-v8a).
    pub fn get_abi(&self) -> anyhow::Result<String> {
        self.controller.get_abi()
    }

    /// Gets the device SDK version (e.g., 30 for Android 11).
    pub fn get_sdk(&self) -> anyhow::Result<String> {
        self.controller.get_sdk()
    }

    /// Performs a click at the specified coordinates (scaled to device resolution).
    pub fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.controller.click(x, y)
    }

    /// Performs a swipe operation.
    pub fn swipe(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
    ) -> anyhow::Result<()> {
        self.controller.swipe(start, end, duration, 0.5, 0.5)
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
}
