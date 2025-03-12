use crossbeam_channel::unbounded;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::sync::Arc;

// START PREPROCESSOR PASTE

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
                ui.painter().galley(
                    text_pos,
                    galley.clone(),
                    Color32::from_rgba_premultiplied(0, 0, 0, alpha),
                );
            }
        });

        if needs_redraw {
            ui.ctx().request_repaint();
        }
    }
}

// END PREPROCESSOR PASTE

pub fn set_native_options() -> eframe::NativeOptions {
    let mut options = eframe::NativeOptions::default();
    options.centered = true;
    options.vsync = true;
    options.renderer = eframe::Renderer::Wgpu;
    options
}

enum RowState {
    Fetching(Option<crossbeam_channel::Receiver<Vec<RowData>>>),
    Displaying(AnimatedRowList),
}

struct App {
    state: RowState,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: RowState::Fetching(None),
        }
    }

    fn start_fetch(&mut self) {
        let (s, r) = unbounded();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let rows = (0..=100)
                .map(|x| RowData::new(format!("GE-Proton-{}", x), format!("/some/path/{}", x)))
                .collect();
            s.send(rows).expect("Failed to send rows");
        });

        self.state = RowState::Fetching(Some(r));
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame();
        puffin::profile_scope!("App::update");

        match &mut self.state {
            RowState::Fetching(receiver_opt) => {
                if let Some(receiver) = receiver_opt {
                    if let Ok(rows) = receiver.try_recv() {
                        self.state = RowState::Displaying(AnimatedRowList::new(
                            rows,
                            ctx.input(|i| i.time),
                            1.0,
                            0.1,
                        ));
                        ctx.request_repaint();
                    } else {
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }
                } else {
                    self.start_fetch();
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Fetching rows");
                });
            }
            RowState::Displaying(row_list) => {
                let mut refresh = false;
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        refresh = ui.button("Refresh").clicked();
                        row_list.show(ui);
                    });
                });
                if refresh {
                    self.start_fetch();
                }
            }
        }
    }
}

fn main() {
    start_puffin_server();
    let options = set_native_options();

    let result = eframe::run_native(
        "Threaded Widget",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    );

    match result {
        Ok(()) => (),
        Err(e) => {
            println!("Failed to exit app properly: {}", e);
        }
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
