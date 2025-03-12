use eframe::egui;
use eframe::epaint::{Color32, Pos2, Rect, Vec2};
use std::sync::Arc;

fn main() {
    start_puffin_server();
    let options = set_native_options();
    eframe::run_native(
        "Animation Widget",
        options,
        Box::new(|cc| Ok(Box::new(AnimationApp::new(cc)))),
    )
    .expect("Failed to run egui application");
}

pub fn set_native_options() -> eframe::NativeOptions {
    let mut options = eframe::NativeOptions::default();
    options.centered = true;
    options.vsync = true;
    options.renderer = eframe::Renderer::Wgpu;
    options
}

#[derive(Default)]
enum Editing {
    #[default]
    VERSION,
    PATH,
    NONE,
}

#[derive(Default)]
struct RowData {
    version: String,
    path: String,
    galley_version: Option<Arc<egui::Galley>>,
    galley_path: Option<Arc<egui::Galley>>,
    editing: Editing,
}

impl RowData {
    fn new(version: String, path: String) -> Self {
        Self {
            version,
            path,
            galley_version: None,
            galley_path: None,
            editing: Editing::NONE,
        }
    }
}

#[derive(Default)]
struct AnimatedRow {
    data: RowData,
    start_time: f64,
    animation_time: f32,
    delay: f32,
}

impl AnimatedRow {
    fn new(row_data: RowData, start_time: f64, duration: f32, delay: f32) -> Self {
        Self {
            data: row_data,
            start_time,
            animation_time: duration,
            delay,
        }
    }

    // Simplified animation progress calculation
    #[inline]
    fn get_progress(&self, time: f64) -> f32 {
        let elapsed = (time - self.start_time - self.delay as f64).max(0.0) as f32;
        let t = (elapsed / self.animation_time).min(1.0);
        egui::emath::easing::quadratic_out(t)
    }
}

#[derive(Default)]
struct AnimatedRowList {
    rows: Vec<AnimatedRow>,
    row_height: f32,
}

impl AnimatedRowList {
    pub fn new(
        rows: Vec<RowData>,
        start_time: f64,
        animation_duration: f32,
        stagger_delay: f32,
    ) -> Self {
        let animated_rows = rows
            .into_iter()
            .enumerate()
            .map(|(i, data)| {
                AnimatedRow::new(
                    data,
                    start_time,
                    animation_duration,
                    i as f32 * stagger_delay,
                )
            })
            .collect();
        Self {
            rows: animated_rows,
            row_height: 60.0,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let time = ui.input(|i| i.time);
        let mut needs_redraw = false;

        ui.vertical(|ui| {
            for row in &mut self.rows {
                ui.horizontal(|ui| {
                    let progress = row.get_progress(time);
                    needs_redraw |= progress < 1.0;

                    let (_id, full_rect) =
                        ui.allocate_space(Vec2::new(ui.available_width(), self.row_height));

                    let half_width = full_rect.width() / 2.0;

                    let start_x = full_rect.left() + half_width;
                    let target_x = full_rect.left();
                    let x_offset = start_x + (target_x - start_x) * progress;

                    let start_x2 = full_rect.right();
                    let target_x2 = start_x;
                    let x_offset2 = start_x2 + (target_x2 - start_x2) * progress;

                    let animated_rect = Rect::from_min_size(
                        Pos2::new(x_offset, full_rect.top()),
                        Vec2::new(half_width, full_rect.height()),
                    );

                    let response = ui.interact(
                        animated_rect,
                        ui.next_auto_id().with(&row.data.version),
                        egui::Sense::click(),
                    );

                    let animated_rect2 = Rect::from_min_size(
                        Pos2::new(x_offset2, full_rect.top()),
                        Vec2::new(half_width, full_rect.height()),
                    );

                    let response2 = ui.interact(
                        animated_rect2,
                        ui.next_auto_id().with(&row.data.path),
                        egui::Sense::click(),
                    );

                    let alpha = (255.0 * progress) as u8;

                    // Direct painting to avoid allocations
                    ui.painter().rect_filled(
                        animated_rect,
                        0.0, // Keep your corner radius
                        Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                    );

                    ui.painter().rect_filled(
                        animated_rect2,
                        0.0,
                        Color32::from_rgba_unmultiplied(180, 180, 180, alpha),
                    );

                    // Cache and reuse text galley
                    let galley = row.data.galley_version.get_or_insert_with(|| {
                        ui.painter().layout_no_wrap(
                            row.data.version.clone(),
                            egui::FontId::new(20.0, egui::FontFamily::Proportional),
                            Color32::BLACK,
                        )
                    });

                    let galley2 = row.data.galley_path.get_or_insert_with(|| {
                        ui.painter().layout_no_wrap(
                            row.data.path.clone(),
                            egui::FontId::new(20.0, egui::FontFamily::Proportional),
                            Color32::BLACK,
                        )
                    });

                    let text_pos = Pos2::new(
                        x_offset + animated_rect.width() * 0.34,
                        animated_rect.top() + animated_rect.height() * 0.3,
                    );

                    let text_pos2 = Pos2::new(
                        x_offset2 + animated_rect2.width() * 0.5,
                        animated_rect2.top() + animated_rect2.height() * 0.3,
                    );

                    ui.painter().galley_with_override_text_color(
                        text_pos,
                        galley.clone(),
                        Color32::from_rgba_premultiplied(0, 0, 0, alpha),
                    );

                    ui.painter().galley_with_override_text_color(
                        text_pos2,
                        galley2.clone(),
                        Color32::from_rgba_premultiplied(0, 0, 0, alpha),
                    );

                    if response.clicked() && !row.data.popup_open {
                        row.data.editing = Editing::VERSION;
                    }

                    if response2.clicked() {
                        row.data.editing = Editing::PATH;
                    }
                });
            }
        });

        if needs_redraw {
            ui.ctx().request_repaint();
        }
    }
}

#[derive(Default)]
struct AnimationApp {
    row_list: AnimatedRowList,
    popup_open: bool,
}

impl AnimationApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut rows = Vec::with_capacity(101);
        for x in 0..=100 {
            rows.push(RowData::new(
                format!("GE-Proton9-{}", x),
                format!("/some/path/{}", x),
            ));
        }
        Self {
            row_list: AnimatedRowList::new(rows, cc.egui_ctx.input(|i| i.time), 1.0, 0.1),
            popup_open: false,
        }
    }
}

impl eframe::App for AnimationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame();
        puffin::profile_scope!("AnimationApp::update");
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.row_list.show(ui);
            });
        });
    }
}

fn start_puffin_server() {
    puffin::set_scopes_on(true);
    if let Ok(puffin_server) = puffin_http::Server::new("127.0.0.1:8585") {
        std::process::Command::new("puffin_viewer")
            .arg("--url")
            .arg("127.0.0.1:8585")
            .spawn()
            .ok();
        std::mem::forget(puffin_server);
    }
}
