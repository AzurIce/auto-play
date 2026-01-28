use std::{sync::Arc, thread, time::Duration};

use enigo::{Axis, Button, Coordinate, Enigo, Mouse, Settings};
use parking_lot::Mutex;
use tracing::info;
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings as CaptureSettings,
    },
    window::Window,
};

use crate::Controller;

/// Frame data captured from the window
struct FrameData {
    image: image::RgbaImage,
    width: u32,
    height: u32,
}

/// Shared state between capture thread and controller
struct SharedCaptureState {
    /// The latest captured frame (Arc to avoid cloning ~8MB image data)
    latest_frame: Option<Arc<FrameData>>,
    /// Whether capture should stop
    should_stop: bool,
    /// Capture error, if any
    error: Option<String>,
}

impl Default for SharedCaptureState {
    fn default() -> Self {
        Self {
            latest_frame: None,
            should_stop: false,
            error: None,
        }
    }
}

/// Context passed to the capture handler
#[derive(Clone)]
struct CaptureContext {
    state: Arc<Mutex<SharedCaptureState>>,
}

/// Handler for windows-capture
struct CaptureHandler {
    context: CaptureContext,
}

impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = CaptureContext;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            context: context.flags,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let mut state = self.context.state.lock();

        if state.should_stop {
            capture_control.stop();
            return Ok(());
        }

        let width = frame.width();
        let height = frame.height();

        let mut buffer = frame.buffer()?;
        let buffer_data: Vec<u8> = buffer.as_nopadding_buffer()?.to_vec();

        if let Some(image) = image::RgbaImage::from_raw(width, height, buffer_data) {
            // Always overwrite with the latest frame (Arc avoids cloning on read)
            state.latest_frame = Some(Arc::new(FrameData {
                image,
                width,
                height,
            }));
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        info!("Capture closed");
        Ok(())
    }
}

/// A Windows controller for window capture and input simulation.
pub struct WindowsController {
    window: Window,
    window_title: String,
    enigo: Arc<Mutex<Enigo>>,
    capture_state: Arc<Mutex<SharedCaptureState>>,
}

impl WindowsController {
    /// Create a new controller by window title (exact match).
    pub fn from_window_title(title: &str) -> anyhow::Result<Self> {
        let windows =
            Window::enumerate().map_err(|e| anyhow::anyhow!("Failed to enumerate windows: {e}"))?;

        let window = windows
            .into_iter()
            .find(|w| w.title().map(|t| t == title).unwrap_or(false))
            .ok_or_else(|| anyhow::anyhow!("Window with title '{}' not found", title))?;

        Self::from_window(window)
    }

    /// Create a new controller from a Window instance.
    pub fn from_window(window: Window) -> anyhow::Result<Self> {
        let window_title = window
            .title()
            .map_err(|e| anyhow::anyhow!("Failed to get window title: {e}"))?;

        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Failed to create enigo instance: {e}"))?;

        let capture_state = Arc::new(Mutex::new(SharedCaptureState::default()));

        // Start capture and wait for first frame to ensure capture works
        Self::start_capture_and_wait(&window, &capture_state)?;

        Ok(Self {
            window,
            window_title,
            enigo: Arc::new(Mutex::new(enigo)),
            capture_state,
        })
    }

    /// Enumerate all available windows
    pub fn enumerate_windows() -> anyhow::Result<Vec<(String, Window)>> {
        let windows =
            Window::enumerate().map_err(|e| anyhow::anyhow!("Failed to enumerate windows: {e}"))?;

        let result: Vec<(String, Window)> = windows
            .into_iter()
            .filter_map(|w| {
                w.title()
                    .ok()
                    .and_then(|t| if !t.is_empty() { Some((t, w)) } else { None })
            })
            .collect();

        Ok(result)
    }

    /// Start the window capture and wait for the first frame.
    fn start_capture_and_wait(
        window: &Window,
        capture_state: &Arc<Mutex<SharedCaptureState>>,
    ) -> anyhow::Result<()> {
        // Reset state
        {
            let mut state = capture_state.lock();
            *state = SharedCaptureState::default();
        }

        let context = CaptureContext {
            state: capture_state.clone(),
        };
        let window = window.clone();

        thread::spawn(move || {
            let settings = CaptureSettings::new(
                window,
                CursorCaptureSettings::Default,
                DrawBorderSettings::Default,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Rgba8,
                context.clone(),
            );

            if let Err(e) = CaptureHandler::start(settings) {
                let err_msg = format!("{e}");
                tracing::error!("Capture error: {}", err_msg);
                context.state.lock().error = Some(err_msg);
            }
        });

        // Wait for the first frame
        let startup_timeout = Duration::from_millis(2000);
        let start = std::time::Instant::now();
        while start.elapsed() < startup_timeout {
            {
                let state = capture_state.lock();
                if let Some(err) = &state.error {
                    return Err(anyhow::anyhow!("Capture failed to start: {err}"));
                }
                if state.latest_frame.is_some() {
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        // Check for error after timeout
        {
            let state = capture_state.lock();
            if let Some(err) = &state.error {
                return Err(anyhow::anyhow!("Capture error: {err}"));
            }
        }

        Err(anyhow::anyhow!(
            "First frame not received within {}ms",
            startup_timeout.as_millis()
        ))
    }

    /// Stop the window capture
    fn stop_capture(&self) {
        let mut state = self.capture_state.lock();
        state.should_stop = true;
    }

    /// Get the window title
    pub fn window_title(&self) -> &str {
        &self.window_title
    }

    /// Get the capture error, if any
    pub fn capture_error(&self) -> Option<String> {
        self.capture_state.lock().error.clone()
    }

    /// Get the current window rect from the OS (always up-to-date).
    fn window_rect(&self) -> anyhow::Result<(i32, i32)> {
        let rect = self
            .window
            .rect()
            .map_err(|e| anyhow::anyhow!("Failed to get window rect: {e}"))?;
        Ok((rect.left, rect.top))
    }

    /// Convert local coordinates to screen coordinates
    fn local_to_screen(&self, x: u32, y: u32) -> anyhow::Result<(i32, i32)> {
        let (ox, oy) = self.window_rect()?;
        Ok((x as i32 + ox, y as i32 + oy))
    }

    /// Get a reference to the latest frame (cheap Arc::clone, no image data copy).
    fn get_latest_frame(&self) -> Option<Arc<FrameData>> {
        let state = self.capture_state.lock();
        state.latest_frame.as_ref().map(Arc::clone)
    }

    // ===== Windows-specific methods =====

    /// Scroll the mouse wheel
    pub fn scroll(&self, x: u32, y: u32, delta: i32) -> anyhow::Result<()> {
        let (screen_x, screen_y) = self.local_to_screen(x, y)?;

        let mut enigo = self.enigo.lock();
        enigo
            .move_mouse(screen_x, screen_y, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("Failed to move mouse: {e}"))?;

        thread::sleep(Duration::from_millis(10));

        enigo
            .scroll(delta, Axis::Vertical)
            .map_err(|e| anyhow::anyhow!("Failed to scroll: {e}"))?;

        Ok(())
    }
}

impl Controller for WindowsController {
    fn screen_size(&self) -> (u32, u32) {
        self.get_latest_frame()
            .map(|f| (f.width, f.height))
            .unwrap_or((1920, 1080))
    }

    fn screencap_raw(&self) -> anyhow::Result<(u32, u32, Vec<u8>)> {
        if let Some(err) = self.capture_error() {
            return Err(anyhow::anyhow!("Capture error: {err}"));
        }

        let frame = self
            .get_latest_frame()
            .ok_or_else(|| anyhow::anyhow!("No frame available"))?;

        Ok((frame.width, frame.height, frame.image.clone().into_raw()))
    }

    fn screencap(&self) -> anyhow::Result<image::DynamicImage> {
        if let Some(err) = self.capture_error() {
            return Err(anyhow::anyhow!("Capture error: {err}"));
        }

        let frame = self
            .get_latest_frame()
            .ok_or_else(|| anyhow::anyhow!("No frame available"))?;

        Ok(image::DynamicImage::ImageRgba8(frame.image.clone()))
    }

    fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
        let (screen_x, screen_y) = self.local_to_screen(x, y)?;

        let mut enigo = self.enigo.lock();
        enigo
            .move_mouse(screen_x, screen_y, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("Failed to move mouse: {e}"))?;

        thread::sleep(Duration::from_millis(10));

        enigo
            .button(Button::Left, enigo::Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to click: {e}"))?;

        Ok(())
    }

    fn swipe(
        &self,
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> anyhow::Result<()> {
        const SWIPE_DELAY_MS: u32 = 5;

        let (ox, oy) = self.window_rect()?;
        let (start_screen_x, start_screen_y) = (start.0 as i32 + ox, start.1 as i32 + oy);

        let mut enigo = self.enigo.lock();

        enigo
            .move_mouse(start_screen_x, start_screen_y, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("Failed to move mouse: {e}"))?;

        thread::sleep(Duration::from_millis(10));

        enigo
            .button(Button::Left, enigo::Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press mouse button: {e}"))?;

        let cubic_spline = |slope_0: f32, slope_1: f32, t: f32| -> f32 {
            let a = slope_0;
            let b = -(2.0 * slope_0 + slope_1 - 3.0);
            let c = -(-slope_0 - slope_1 + 2.0);
            a * t + b * t.powi(2) + c * t.powi(3)
        };

        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

        let duration_ms = duration.as_millis() as u32;
        for t in (SWIPE_DELAY_MS..duration_ms).step_by(SWIPE_DELAY_MS as usize) {
            let progress = cubic_spline(slope_in, slope_out, t as f32 / duration_ms as f32);
            let progress = progress.clamp(0.0, 1.0);

            let cur_x = lerp(start.0 as f32, end.0 as f32, progress) as i32;
            let cur_y = lerp(start.1 as f32, end.1 as f32, progress) as i32;

            enigo
                .move_mouse(cur_x + ox, cur_y + oy, Coordinate::Abs)
                .map_err(|e| anyhow::anyhow!("Failed to move mouse during swipe: {e}"))?;

            thread::sleep(Duration::from_millis(SWIPE_DELAY_MS as u64));
        }

        enigo
            .move_mouse(end.0 + ox, end.1 + oy, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("Failed to move mouse to end position: {e}"))?;

        thread::sleep(Duration::from_millis(50));

        enigo
            .button(Button::Left, enigo::Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release mouse button: {e}"))?;

        Ok(())
    }
}

impl Drop for WindowsController {
    fn drop(&mut self) {
        self.stop_capture();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::EnvFilter;

    fn init_tracing_subscriber() {
        let _ = tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive("TRACE".parse().unwrap())
                    .from_env()
                    .unwrap(),
            )
            .try_init();
    }

    #[test]
    fn test_enumerate_windows() {
        init_tracing_subscriber();

        let windows = WindowsController::enumerate_windows().unwrap();
        for (title, _window) in windows.iter().take(10) {
            println!("Window: {}", title);
        }
        assert!(!windows.is_empty());
    }

    #[test]
    fn test_screencap() {
        init_tracing_subscriber();

        let controller = WindowsController::from_window_title("Notepad").unwrap();

        // Wait a bit for frames to arrive
        thread::sleep(Duration::from_millis(100));

        let screen = controller.screencap().unwrap();
        println!("Screenshot: {}x{}", screen.width(), screen.height());
        screen.save("windows_cap.png").unwrap();

        let screen_scaled = controller.screencap_scaled().unwrap();
        println!(
            "Scaled: {}x{}",
            screen_scaled.width(),
            screen_scaled.height()
        );
        screen_scaled.save("windows_cap_scaled.png").unwrap();
    }

    #[test]
    fn test_click() {
        init_tracing_subscriber();

        let controller = WindowsController::from_window_title("Endfield").unwrap();
        controller.click(100, 100).unwrap();
        // thread::sleep(Duration::from_millis(100));
        // controller.click(200, 200).unwrap();
    }

    #[test]
    fn test_swipe() {
        init_tracing_subscriber();

        let controller = WindowsController::from_window_title("Notepad").unwrap();
        controller
            .swipe((100, 100), (300, 300), Duration::from_millis(500), 0.5, 0.5)
            .unwrap();
    }
}
