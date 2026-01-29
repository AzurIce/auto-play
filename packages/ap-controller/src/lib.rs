use std::{any::Any, time::Duration};

pub use enigo::Key;
use image::math::Rect;

pub mod android;

#[cfg(feature = "windows")]
pub mod windows;

// Re-export controllers for convenience
pub use android::AndroidController;

#[cfg(feature = "windows")]
pub use windows::WindowsController;

/// Default reference height for coordinate scaling (1080p)
pub const DEFAULT_HEIGHT: u32 = 1080;

/// A trait for device/window controllers that provide screen capture and input simulation.
///
/// This trait abstracts common operations across different platforms (Android, Windows, etc.),
/// allowing for platform-agnostic automation code.
pub trait ControllerTrait {
    // ===== Screen Information =====

    /// Get the screen/window size as (width, height)
    fn screen_size(&self) -> (u32, u32);

    /// Get the scale factor from the device's resolution to 1920x1080.
    ///
    /// Formula: `device_height / 1080`
    ///
    /// This is used for coordinate scaling - coordinates in 1920x1080 space
    /// can be converted to device coordinates by dividing by this factor.
    fn scale_factor(&self) -> f32 {
        self.screen_size().1 as f32 / DEFAULT_HEIGHT as f32
    }

    // ===== Screenshot Methods =====

    /// Get the raw screenshot data as (width, height, rgba_bytes)
    fn screencap_raw(&self) -> anyhow::Result<(u32, u32, Vec<u8>)>;

    /// Get the decoded screenshot as a DynamicImage
    fn screencap(&self) -> anyhow::Result<image::DynamicImage>;

    /// Get a screenshot scaled to DEFAULT_HEIGHT (1080p).
    ///
    /// This is useful for template matching with templates designed for 1080p.
    fn screencap_scaled(&self) -> anyhow::Result<image::DynamicImage> {
        let screen = self.screencap()?;

        if screen.height() != DEFAULT_HEIGHT {
            let scale_factor = DEFAULT_HEIGHT as f32 / screen.height() as f32;
            let new_width = (screen.width() as f32 * scale_factor) as u32;
            let new_height = (screen.height() as f32 * scale_factor) as u32;

            Ok(image::DynamicImage::from(image::imageops::resize(
                &screen,
                new_width,
                new_height,
                image::imageops::FilterType::Triangle,
            )))
        } else {
            Ok(screen)
        }
    }

    // ===== Click Methods =====

    /// Click at the specified coordinates
    fn click(&self, x: u32, y: u32) -> anyhow::Result<()>;

    /// Click at coordinates scaled from 1920x1080 to actual resolution.
    ///
    /// This allows writing automation code in 1920x1080 coordinates
    /// that works on any resolution.
    fn click_scaled(&self, x_scaled: u32, y_scaled: u32) -> anyhow::Result<()> {
        let scale_factor = self.scale_factor();
        let x = (x_scaled as f32 / scale_factor) as u32;
        let y = (y_scaled as f32 / scale_factor) as u32;
        self.click(x, y)
    }

    /// Click at a random position within the given rectangle
    fn click_in_rect(&self, rect: Rect) -> anyhow::Result<()> {
        let x = rand::random::<u32>() % rect.width + rect.x;
        let y = rand::random::<u32>() % rect.height + rect.y;
        self.click(x, y)
    }

    /// Click in a rectangle with coordinates scaled from 1920x1080
    fn click_in_rect_scaled(&self, rect_scaled: Rect) -> anyhow::Result<()> {
        let scale_factor = self.scale_factor();
        let rect = Rect {
            x: (rect_scaled.x as f32 / scale_factor) as u32,
            y: (rect_scaled.y as f32 / scale_factor) as u32,
            width: (rect_scaled.width as f32 / scale_factor) as u32,
            height: (rect_scaled.height as f32 / scale_factor) as u32,
        };
        self.click_in_rect(rect)
    }

    // ===== Swipe Methods =====

    /// Perform a swipe gesture from start to end.
    ///
    /// # Arguments
    /// * `start` - Starting position (x, y)
    /// * `end` - Ending position (x, y)
    /// * `duration` - Duration of the swipe
    /// * `slope_in` - Starting slope for cubic interpolation (controls acceleration)
    /// * `slope_out` - Ending slope for cubic interpolation (controls deceleration)
    fn swipe(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()>;

    /// Perform a swipe with coordinates scaled from 1920x1080
    fn swipe_scaled(
        &self,
        start_scaled: (u32, u32),
        end_scaled: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        let scale_factor = self.scale_factor();
        let start = (
            (start_scaled.0 as f32 / scale_factor) as u32,
            (start_scaled.1 as f32 / scale_factor) as u32,
        );
        let end = (
            (end_scaled.0 as f32 / scale_factor) as i32,
            (end_scaled.1 as f32 / scale_factor) as i32,
        );
        self.swipe(start, end, duration, slope_in, slope_out)
    }

    fn press(&self, key: Key) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::EnvFilter;
    pub fn init_tracing_subscriber() {
        let _ = tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive("TRACE".parse().unwrap())
                    .from_env()
                    .unwrap(),
            )
            .try_init();
    }
}

pub trait AnyControllerTrait: Any + Send + ControllerTrait {}
impl<T: ControllerTrait + Any + Send> AnyControllerTrait for T {}

pub struct Controller {
    inner: Box<dyn AnyControllerTrait>,
}

impl ControllerTrait for Controller {
    fn screen_size(&self) -> (u32, u32) {
        self.inner.screen_size()
    }

    fn screencap_raw(&self) -> anyhow::Result<(u32, u32, Vec<u8>)> {
        self.inner.screencap_raw()
    }

    fn screencap(&self) -> anyhow::Result<image::DynamicImage> {
        self.inner.screencap()
    }

    fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.inner.click(x, y)
    }

    fn swipe(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        self.inner.swipe(start, end, duration, slope_in, slope_out)
    }

    fn press(&self, key: Key) -> anyhow::Result<()> {
        self.inner.press(key)
    }
}

impl Controller {
    pub fn new<T: ControllerTrait + Any + Send>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
    pub fn downcast_ref<T: ControllerTrait + 'static>(&self) -> Option<&T> {
        (self.inner.as_ref() as &dyn Any).downcast_ref::<T>()
    }
}
