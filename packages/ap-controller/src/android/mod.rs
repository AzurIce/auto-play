use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ap_adb::command::local_service::Input;

use app::App;
pub mod app;

use crate::Controller;

/// Android controller structure
pub struct AndroidController {
    device: ap_adb::Device,
    width: u32,
    height: u32,
    maa_touch: Arc<Mutex<app::maatouch::MaaTouch>>,
}

impl AndroidController {
    pub fn connect(serial: &str) -> anyhow::Result<Self> {
        let device = ap_adb::connect(serial)?;
        Self::from_device(device)
    }

    pub fn from_device(device: ap_adb::Device) -> anyhow::Result<Self> {
        let screen = device.screencap()?;
        let (width, height) = (screen.width(), screen.height());
        let maa_touch = app::maatouch::MaaTouch::build(&device)?;
        let maa_touch = Arc::new(Mutex::new(maa_touch));
        Ok(Self {
            device,
            width,
            height,
            maa_touch,
        })
    }

    // ===== Android-specific methods =====

    pub fn is_screen_on(&self) -> anyhow::Result<bool> {
        let output = self.device.execute_command_by_socket(
            ap_adb::command::local_service::ShellCommand::new(
                "dumpsys power | grep mWakefulness".to_string(),
            ),
        )?;
        Ok(output.contains("mWakefulness=Awake"))
    }

    pub fn ensure_screen_on(&self) -> anyhow::Result<()> {
        if !self.is_screen_on()? {
            self.device
                .input(Input::Keyevent("KEYCODE_WAKEUP".to_string()))
                .map_err(|err| anyhow::anyhow!("failed to wake up device: {err:?}"))?;
        }
        Ok(())
    }

    pub fn get_abi(&self) -> anyhow::Result<String> {
        let res = self.device.execute_command_by_socket(
            ap_adb::command::local_service::ShellCommand::new(
                "getprop ro.product.cpu.abi".to_string(),
            ),
        )?;
        Ok(res.strip_suffix("\n").unwrap_or(&res).to_string())
    }

    pub fn get_sdk(&self) -> anyhow::Result<String> {
        let res = self.device.execute_command_by_socket(
            ap_adb::command::local_service::ShellCommand::new(
                "getprop ro.build.version.sdk".to_string(),
            ),
        )?;
        Ok(res.strip_suffix("\n").unwrap_or(&res).to_string())
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

    /// Get the underlying ADB device
    pub fn device(&self) -> &ap_adb::Device {
        &self.device
    }
}

impl Controller for AndroidController {
    fn screen_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn screencap_raw(&self) -> anyhow::Result<(u32, u32, Vec<u8>)> {
        self.device
            .screencap_raw()
            .map_err(|err| anyhow::anyhow!("failed to get raw screencap: {err:?}"))
    }

    fn screencap(&self) -> anyhow::Result<image::DynamicImage> {
        self.device
            .screencap()
            .map_err(|err| anyhow::anyhow!("failed to get screencap: {err:?}"))
    }

    fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
        self.maa_touch.lock().unwrap().click(x, y)
    }

    fn swipe(
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
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;
    use crate::tests::init_tracing_subscriber;

    fn test_controller() -> AndroidController {
        let device = ap_adb::connect("192.168.1.3:40919").unwrap();
        AndroidController::from_device(device).unwrap()
    }

    #[test]
    fn test_capture() {
        init_tracing_subscriber();

        let controller = test_controller();
        let screen = controller.screencap().unwrap();
        println!("{}x{}", screen.width(), screen.height());
        screen.save("cap.png").unwrap();
        let screen = controller.screencap_scaled().unwrap();
        println!("{}x{}", screen.width(), screen.height());
        screen.save("cap_scaled.png").unwrap();
    }

    #[test]
    fn test_screen_on() {
        init_tracing_subscriber();

        let controller = test_controller();
        println!("is_screen_on: {}", controller.is_screen_on().unwrap());
        controller.ensure_screen_on().unwrap();
        let is_screen_on = controller.is_screen_on().unwrap();
        println!("is_screen_on: {}", is_screen_on);
        assert!(is_screen_on);
    }

    #[test]
    fn test_click() {
        init_tracing_subscriber();

        let controller = test_controller();
        controller.click(100, 100).unwrap();
        thread::sleep(Duration::from_millis(50));

        controller.click(100, 100).unwrap();
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn test_swipe() {
        init_tracing_subscriber();

        let controller = test_controller();
        controller
            .swipe((100, 100), (200, 200), Duration::from_millis(100), 0.5, 0.5)
            .unwrap();

        thread::sleep(Duration::from_millis(50));
    }
}
