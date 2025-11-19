use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use image::math::Rect;

use crate::app::App;

pub mod app;

/// A controller, responsible for the device operations like click, swipe, screencap, etc.
pub struct Controller {
    device: ap_adb::Device,
    width: u32,
    height: u32,
    maa_touch: Arc<Mutex<app::maatouch::MaaTouch>>,
    // screen_cache: Option<image::DynamicImage>,
}

pub const DEFAULT_HEIGHT: u32 = 1080;

impl Controller {
    pub fn connect(serial: &str) -> anyhow::Result<Self> {
        let device = ap_adb::connect(serial)?;
        Self::from_device(device)
    }
    pub fn from_device(device: ap_adb::Device) -> anyhow::Result<Self> {
        let screen = device.screencap()?;
        let (width, height) = (screen.width(), screen.height());
        let maa_touch = app::maatouch::MaaTouch::init(&device)?;
        let maa_touch = Arc::new(Mutex::new(maa_touch));
        Ok(Self {
            device,
            width,
            height,
            maa_touch,
            // screen_cache: Some(screen),
        })
    }

    pub fn screen_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// A scale factor from the device's resolution to 1920x1080
    /// $device_res * scale_factor = 1920x1080$
    pub fn scale_factor(&self) -> f32 {
        self.screen_size().1 as f32 / DEFAULT_HEIGHT as f32
    }

    pub fn press_home(&self) -> anyhow::Result<()> {
        self.device
            .execute_command_by_process("shell input keyevent HOME")?;
        Ok(())
    }

    pub fn press_esc(&self) -> anyhow::Result<()> {
        self.device
            .execute_command_by_process("shell input keyevent 111")?;
        Ok(())
    }

    pub fn click_in_rect(&self, rect: Rect) -> anyhow::Result<()> {
        let x = rand::random::<u32>() % rect.width + rect.x;
        let y = rand::random::<u32>() % rect.height + rect.y;
        self.click(x, y)
    }

    /// A scaled version of [`Controller::click_in_rect`].
    ///
    /// This scaled the coord from 1920x1080 to the actual size by simply dividing [`Controller::scale_factor`]
    pub fn click_in_rect_scaled(&self, rect_scaled: Rect) -> anyhow::Result<()> {
        let scale_fector = self.scale_factor();
        let rect = Rect {
            x: (rect_scaled.x as f32 / scale_fector) as u32,
            y: (rect_scaled.y as f32 / scale_fector) as u32,
            width: (rect_scaled.width as f32 / scale_fector) as u32,
            height: (rect_scaled.height as f32 / scale_fector) as u32,
        };
        self.click_in_rect(rect)
    }

    pub fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.maa_touch.lock().unwrap().click(x, y)
    }

    /// A scaled version of [`Controller::click`].
    ///
    /// This scaled the coord from 1920x1080 to the actual size by simply dividing [`Controller::scale_factor`]
    pub fn click_scaled(&self, x_scaled: u32, y_scaled: u32) -> anyhow::Result<()> {
        let scale_factor = self.scale_factor();
        let (x, y) = (
            x_scaled as f32 / scale_factor,
            y_scaled as f32 / scale_factor,
        );
        self.click(x as u32, y as u32)
    }

    pub fn swipe(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        self.maa_touch
            .lock()
            .unwrap()
            .swipe(start, end, duration, slope_in, slope_out)
    }

    /// A scaled version of [`Controller::swipe`].
    ///
    /// This scaled the coord from 1920x1080 to the actual size by simply dividing [`Controller::scale_factor`]
    pub fn swipe_scaled(
        &self,
        start_scaled: (u32, u32),
        end_scaled: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        let scale_factor = self.scale_factor();
        let (start, end) = (
            (
                start_scaled.0 as f32 / scale_factor,
                start_scaled.1 as f32 / scale_factor,
            ),
            (
                end_scaled.0 as f32 / scale_factor,
                end_scaled.1 as f32 / scale_factor,
            ),
        );
        self.swipe(
            (start.0 as u32, start.1 as u32),
            (end.0 as i32, end.1 as i32),
            duration,
            slope_in,
            slope_out,
        )
    }

    /// Get the raw screencap data in bytes
    pub fn screencap_raw(&self) -> anyhow::Result<Vec<u8>> {
        self.device
            .raw_screencap()
            .map_err(|err| anyhow::anyhow!("failed to get raw screencap: {err:?}"))
    }

    /// Get the decoded screencap image
    pub fn screencap(&self) -> anyhow::Result<image::DynamicImage> {
        self.device
            .screencap()
            .map_err(|err| anyhow::anyhow!("failed to get screencap: {err:?}"))
    }

    /// A scaled version of [`Controller::swipe`].
    ///
    /// This scaled the screenshot image to [`DEFAULT_HEIGHT`]
    pub fn screencap_scaled(&self) -> anyhow::Result<image::DynamicImage> {
        let screen = self.screencap()?;
        let screen = if screen.height() != DEFAULT_HEIGHT {
            // let scale_factor = 2560.0 / image.width() as f32;
            let scale_factor = DEFAULT_HEIGHT as f32 / screen.height() as f32;

            let new_width = (screen.width() as f32 * scale_factor) as u32;
            let new_height = (screen.height() as f32 * scale_factor) as u32;

            image::DynamicImage::from(image::imageops::resize(
                &screen,
                new_width,
                new_height,
                image::imageops::FilterType::Triangle,
            ))
        } else {
            screen
        };
        Ok(screen)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use tracing_subscriber::EnvFilter;

    use super::*;

    pub fn init_tracing_subscriber() {
        let _ = tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive("TRACE".parse().unwrap())
                    .from_env()
                    .unwrap(),
            )
            .init();
    }

    #[test]
    fn test_controller() {
        init_tracing_subscriber();

        let device = ap_adb::connect("127.0.0.1:16384").unwrap();
        let controller = Controller::from_device(device).unwrap();
        let screen = controller.screencap().unwrap();
        println!("{}x{}", screen.width(), screen.height());
        screen.save("test_screen.png").unwrap();
        let screen = controller.screencap_scaled().unwrap();
        println!("{}x{}", screen.width(), screen.height());
        screen.save("test_screen_scaled.png").unwrap();
    }

    #[test]
    fn test_click() {
        init_tracing_subscriber();

        let device = ap_adb::connect("127.0.0.1:16384").unwrap();
        let controller = Controller::from_device(device).unwrap();
        controller.click(100, 100).unwrap();

        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn test_swipe() {
        init_tracing_subscriber();

        let device = ap_adb::connect("127.0.0.1:16384").unwrap();
        let controller = Controller::from_device(device).unwrap();
        controller
            .swipe((100, 100), (200, 200), Duration::from_millis(100), 0.5, 0.5)
            .unwrap();

        thread::sleep(Duration::from_millis(50));
    }
}
