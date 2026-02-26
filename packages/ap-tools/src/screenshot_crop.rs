//! Screenshot Crop Tool
//!
//! An egui application that:
//! 1. Lists available windows in a dropdown
//! 2. Takes a screenshot of the selected window on button click
//! 3. Displays the screenshot and lets you draw a rectangle to crop
//! 4. Saves the cropped region as a PNG file

use std::thread;
use std::time::Duration;

use ap_controller::windows::WindowsController;
use ap_controller::ControllerTrait;
use eframe::egui;
use image::DynamicImage;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Screenshot Crop Tool"),
        ..Default::default()
    };
    eframe::run_native(
        "Screenshot Crop Tool",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct App {
    /// Available window titles
    window_titles: Vec<String>,
    /// Currently selected window index
    selected_window: usize,
    /// Status message
    status: String,

    /// The captured screenshot as DynamicImage (for cropping)
    screenshot_image: Option<DynamicImage>,
    /// The egui texture handle for display
    screenshot_texture: Option<egui::TextureHandle>,
    /// Original image dimensions
    image_size: (u32, u32),

    /// Drag state for rectangle selection (in image coordinates)
    drag_start: Option<egui::Pos2>,
    drag_end: Option<egui::Pos2>,
    /// Finalized selection rectangle (in image coordinates)
    selection: Option<[f32; 4]>, // [x, y, w, h]

    /// Save file name
    save_name: String,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let window_titles = match WindowsController::enumerate_windows() {
            Ok(windows) => windows.into_iter().map(|(title, _)| title).collect(),
            Err(e) => {
                eprintln!("Failed to enumerate windows: {e}");
                vec![]
            }
        };

        Self {
            window_titles,
            selected_window: 0,
            status: "Select a window and click 'Screenshot'".into(),
            screenshot_image: None,
            screenshot_texture: None,
            image_size: (0, 0),
            drag_start: None,
            drag_end: None,
            selection: None,
            save_name: "template.png".into(),
        }
    }

    fn take_screenshot(&mut self, ctx: &egui::Context) {
        if self.window_titles.is_empty() {
            self.status = "No windows available".into();
            return;
        }

        let title = &self.window_titles[self.selected_window];
        self.status = format!("Capturing '{title}'...");

        match WindowsController::from_window_title(title) {
            Ok(controller) => {
                thread::sleep(Duration::from_millis(200));
                match controller.screencap() {
                    Ok(img) => {
                        let (w, h) = (img.width(), img.height());
                        self.image_size = (w, h);

                        // Convert to egui texture
                        let rgba = img.to_rgba8();
                        let pixels = rgba.as_flat_samples();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize],
                            pixels.as_slice(),
                        );
                        self.screenshot_texture = Some(ctx.load_texture(
                            "screenshot",
                            color_image,
                            egui::TextureOptions::LINEAR,
                        ));
                        self.screenshot_image = Some(img);
                        self.selection = None;
                        self.drag_start = None;
                        self.drag_end = None;
                        self.status = format!("Captured {w}x{h} - Draw a rectangle to crop");
                    }
                    Err(e) => {
                        self.status = format!("Screenshot failed: {e}");
                    }
                }
            }
            Err(e) => {
                self.status = format!("Failed to connect: {e}");
            }
        }
    }

    fn save_crop(&mut self) {
        let Some(ref img) = self.screenshot_image else {
            self.status = "No screenshot to crop".into();
            return;
        };
        let Some([x, y, w, h]) = self.selection else {
            self.status = "No region selected".into();
            return;
        };

        let (x, y, w, h) = (x as u32, y as u32, w as u32, h as u32);
        if w == 0 || h == 0 {
            self.status = "Selection too small".into();
            return;
        }

        let cropped = img.crop_imm(x, y, w, h);

        // Use rfd to pick save location
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&self.save_name)
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .set_directory("assets/ff14")
            .save_file()
        {
            match cropped.save(&path) {
                Ok(()) => {
                    self.status = format!("Saved {}x{} to {}", w, h, path.display());
                }
                Err(e) => {
                    self.status = format!("Save failed: {e}");
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel: controls
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Window:");
                egui::ComboBox::from_id_salt("window_select")
                    .selected_text(
                        self.window_titles
                            .get(self.selected_window)
                            .cloned()
                            .unwrap_or_default(),
                    )
                    .width(300.0)
                    .show_ui(ui, |ui| {
                        for (i, title) in self.window_titles.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_window, i, title);
                        }
                    });

                if ui.button("🔄 Refresh").clicked() {
                    if let Ok(windows) = WindowsController::enumerate_windows() {
                        self.window_titles = windows.into_iter().map(|(title, _)| title).collect();
                    }
                }

                if ui.button("📷 Screenshot").clicked() {
                    self.take_screenshot(ctx);
                }

                ui.separator();

                ui.label("Save as:");
                ui.text_edit_singleline(&mut self.save_name);

                let has_selection = self.selection.is_some();
                if ui
                    .add_enabled(has_selection, egui::Button::new("💾 Save Crop"))
                    .clicked()
                {
                    self.save_crop();
                }
            });

            // Status bar
            ui.horizontal(|ui| {
                ui.label(&self.status);
                if let Some([x, y, w, h]) = self.selection {
                    ui.separator();
                    ui.label(format!(
                        "Selection: ({}, {}) {}x{}",
                        x as u32, y as u32, w as u32, h as u32
                    ));
                }
            });
        });

        // Central panel: image display with drag-to-select
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref texture) = self.screenshot_texture {
                egui::ScrollArea::both().show(ui, |ui| {
                    let img_w = self.image_size.0 as f32;
                    let img_h = self.image_size.1 as f32;

                    // Fit image to available width, maintain aspect ratio
                    let available_w = ui.available_width();
                    let scale = (available_w / img_w).min(1.0);
                    let display_w = img_w * scale;
                    let display_h = img_h * scale;

                    let (response, painter) =
                        ui.allocate_painter(egui::vec2(display_w, display_h), egui::Sense::drag());

                    let rect = response.rect;

                    // Draw the screenshot
                    painter.image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );

                    // Handle drag for rectangle selection
                    if response.drag_started() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            self.drag_start = Some(pos);
                            self.drag_end = Some(pos);
                            self.selection = None;
                        }
                    }

                    if response.dragged() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            self.drag_end = Some(pos);
                        }
                    }

                    if response.drag_stopped() {
                        // Finalize selection: convert screen coords to image coords
                        if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
                            let to_img = |screen_pos: egui::Pos2| -> (f32, f32) {
                                let x = ((screen_pos.x - rect.min.x) / scale).clamp(0.0, img_w);
                                let y = ((screen_pos.y - rect.min.y) / scale).clamp(0.0, img_h);
                                (x, y)
                            };
                            let (x1, y1) = to_img(start);
                            let (x2, y2) = to_img(end);
                            let x = x1.min(x2);
                            let y = y1.min(y2);
                            let w = (x1 - x2).abs();
                            let h = (y1 - y2).abs();
                            if w > 2.0 && h > 2.0 {
                                self.selection = Some([x, y, w, h]);
                            }
                        }
                    }

                    // Draw selection rectangle
                    let draw_rect = |start: egui::Pos2, end: egui::Pos2| {
                        let sel_rect = egui::Rect::from_two_pos(start, end);
                        // Dim area outside selection
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.max.x, sel_rect.min.y),
                            ),
                            0.0,
                            egui::Color32::from_black_alpha(100),
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(rect.min.x, sel_rect.max.y),
                                rect.max,
                            ),
                            0.0,
                            egui::Color32::from_black_alpha(100),
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(rect.min.x, sel_rect.min.y),
                                egui::pos2(sel_rect.min.x, sel_rect.max.y),
                            ),
                            0.0,
                            egui::Color32::from_black_alpha(100),
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(sel_rect.max.x, sel_rect.min.y),
                                egui::pos2(rect.max.x, sel_rect.max.y),
                            ),
                            0.0,
                            egui::Color32::from_black_alpha(100),
                        );
                        // Draw border
                        painter.rect_stroke(
                            sel_rect,
                            0.0,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
                            egui::StrokeKind::Outside,
                        );
                    };

                    // Draw active drag
                    if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
                        if self.selection.is_none() {
                            draw_rect(start, end);
                        }
                    }

                    // Draw finalized selection
                    if let Some([x, y, w, h]) = self.selection {
                        let screen_start =
                            egui::pos2(rect.min.x + x * scale, rect.min.y + y * scale);
                        let screen_end =
                            egui::pos2(rect.min.x + (x + w) * scale, rect.min.y + (y + h) * scale);
                        draw_rect(screen_start, screen_end);
                    }
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No screenshot yet. Select a window and click 'Screenshot'.");
                });
            }
        });
    }
}
