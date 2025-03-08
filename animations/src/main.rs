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
struct RowData {
    version: String,
    path: String,
    text_galley: Option<Arc<egui::Galley>>, // Cached text layout
}

impl RowData {
    fn new(version: String, path: String) -> Self {
        Self {
            version,
            path,
            text_galley: None,
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
        -t * (t - 2.0) // Inline quadratic out easin
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

    #[inline]
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let time = ui.input(|i| i.time);
        let mut needs_redraw = false;

        ui.vertical(|ui| {
            for row in &mut self.rows {
                let progress = row.get_progress(time);
                needs_redraw |= progress < 1.0;

                let (_id, rect) =
                    ui.allocate_space(Vec2::new(ui.available_width(), self.row_height));

                let start_x = rect.right();
                let target_x = rect.left();
                let x_offset = start_x + (target_x - start_x) * progress;
                let animated_rect = Rect::from_min_size(
                    Pos2::new(x_offset, rect.top()),
                    Vec2::new(rect.width(), rect.height()),
                );
                let alpha = (255.0 * progress) as u8;

                // Direct painting to avoid allocations
                ui.painter().rect_filled(
                    animated_rect,
                    100.0, // Keep your corner radius
                    Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                );

                // Cache and reuse text galley
                let galley = row.data.text_galley.get_or_insert_with(|| {
                    ui.painter().layout_no_wrap(
                        row.data.version.clone(),
                        egui::FontId::new(20.0, egui::FontFamily::Proportional),
                        Color32::BLACK,
                    )
                });
                let text_pos = Pos2::new(
                    x_offset + rect.width() * 0.48,
                    rect.top() + rect.height() * 0.3,
                );
                ui.painter().galley_with_color(
                    text_pos,
                    galley.clone(),
                    Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
                );
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

// Simplified puffin server start (optional, removed by default for performance)
fn start_puffin_server() {
    // Uncomment if profiling is needed

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
