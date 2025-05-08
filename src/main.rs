use std::{
    io::{self, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use image::{ImageBuffer, Rgba};
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    encoder::{AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};

// Handles capture events.
struct Capture {
    // The video encoder that will be used to encode the frames.
    encoder: Option<VideoEncoder>,
    // To measure the time the capture has been running
    start: Instant,
    need_update: Arc<AtomicBool>,
    img_buf: Arc<Mutex<ImageBuffer<Rgba<u8>, Vec<u8>>>>,
}

impl GraphicsCaptureApiHandler for Capture {
    // The type of flags used to get the values from the settings.
    type Flags = (Arc<AtomicBool>, Arc<Mutex<ImageBuffer<Rgba<u8>, Vec<u8>>>>);

    // The type of error that can be returned from `CaptureControl` and `start` functions.
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function that will be called to create a new instance. The flags can be passed from settings.
    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        println!("Created");

        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(1920, 1080),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            "video.mp4",
        )?;

        Ok(Self {
            encoder: Some(encoder),
            start: Instant::now(),
            need_update: ctx.flags.0,
            img_buf: ctx.flags.1,
        })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        print!(
            "\rRecording for: {} seconds",
            self.start.elapsed().as_secs()
        );
        io::stdout().flush()?;

        // Send the frame to the video encoder
        self.encoder.as_mut().unwrap().send_frame(frame)?;

        // Note: The frame has other uses too, for example, you can save a single frame to a file, like this:
        // frame.save_as_image("frame.png", ImageFormat::Png)?;
        // Or get the raw data like this so you have full control:
        // let data = frame.buffer()?;
        if self.need_update.load(Ordering::Relaxed) {
            let mut img_buf = self.img_buf.lock().unwrap();
            if img_buf.width() != frame.width() || img_buf.height() != frame.height() {
                *img_buf = ImageBuffer::from_raw(
                    frame.width(),
                    frame.height(),
                    frame
                        .buffer()
                        .unwrap()
                        .as_nopadding_buffer()
                        .unwrap()
                        .to_vec(),
                )
                .unwrap();
            } else {
                let mut buf = frame.buffer().unwrap();
                img_buf.copy_from_slice(buf.as_nopadding_buffer().unwrap());
            }
            self.need_update.store(false, Ordering::Release);
        }

        // Stop the capture after 6 seconds
        if self.start.elapsed().as_secs() >= 6 {
            // Finish the encoder and save the video.
            self.encoder.take().unwrap().finish()?;

            capture_control.stop();

            // Because there wasn't any new lines in previous prints
            println!();
        }

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) closes.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture session ended");

        Ok(())
    }
}

fn main() {
    // Gets the foreground window, refer to the docs for other capture items
    let primary_monitor = Monitor::primary().expect("There is no primary monitor");

    let need_update = Arc::new(AtomicBool::new(true));
    let img_buf = Arc::new(Mutex::new(ImageBuffer::new(1920, 1080)));
    let settings = Settings::new(
        // Item to capture
        primary_monitor,
        // Capture cursor settings
        CursorCaptureSettings::Default,
        // Draw border settings
        DrawBorderSettings::Default,
        // The desired color format for the captured frame.
        ColorFormat::Rgba8,
        // Additional flags for the capture settings that will be passed to user defined `new` function.
        (need_update.clone(), img_buf.clone()),
    );

    let t = thread::spawn(move || {
        // Starts the capture and takes control of the current thread.
        // The errors from handler trait will end up here
        Capture::start(settings).expect("Screen capture failed");
    });

    need_update.store(true, Ordering::Relaxed);
    while need_update.load(Ordering::Relaxed) {}
    img_buf.lock().unwrap().save("frame.png").unwrap();

    thread::sleep(Duration::from_secs(2));
    need_update.store(true, Ordering::Relaxed);
    while need_update.load(Ordering::Relaxed) {}
    img_buf.lock().unwrap().save("frame.png").unwrap();

    // loop {
    //     thread::sleep(Duration::from_millis(500));
    //     println!("{:?}", need_update);
    //     need_update.store(true, Ordering::Relaxed);
    // }

    t.join().unwrap();
}
