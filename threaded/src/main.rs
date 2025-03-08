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

struct App {
    sql_sender: crossbeam_channel::Sender<Vec<RowData>>,
    sql_receiver: crossbeam_channel::Receiver<Vec<RowData>>,
    fetch_thread: Option<std::thread::JoinHandle<()>>,
    current_page: u32,
    poll: bool,
    rows: Option<AnimatedRowList>,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (s, r) = unbounded();
        Self {
            sql_sender: s,
            sql_receiver: r,
            fetch_thread: None,
            current_page: 1,
            poll: true,
            rows: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame();
        puffin::profile_scope!("App::update");
        // We need to retrieve some rows
        if self.fetch_thread.is_none() && self.poll {
            let func = move |cloned_sender: crossbeam_channel::Sender<Vec<RowData>>, page: u32| {
                delayed_fetch_operation(page, &cloned_sender);
            };
            let cloned_sender = self.sql_sender.clone();
            let page = self.current_page;
            let handle = std::thread::spawn(move || func(cloned_sender, page));
            self.fetch_thread = Some(handle);
        }

        // If we find some rows, display them. If not, say we're fetching them
        if let Some(row_list) = &mut self.rows {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    row_list.show(ui);
                });
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Fetching rows");
            });
        }

        // Check if the sender has finished
        if self.poll {
            match self.sql_receiver.try_recv() {
                Ok(t) => {
                    self.rows = Some(AnimatedRowList::new(t, ctx.input(|i| i.time), 1.0, 0.1));
                    self.poll = false;
                    if let Some(handle) = self.fetch_thread.take() {
                        let _ = handle.join();
                        self.fetch_thread = None;
                    }
                }
                Err(_) => {}
            };
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn delayed_fetch_operation(_page: u32, sender: &crossbeam_channel::Sender<Vec<RowData>>) {
    let count = 5;
    for _ in (0..count).rev() {
        std::thread::sleep(std::time::Duration::new(1, 0));
    }

    let mut v: Vec<RowData> = Vec::new();
    for x in 0..100 {
        let row: RowData = RowData {
            version: String::from(format!("ProtonGE-{}", x)),
            path: String::from(format!("some/path/{}", x)),
            text_galley: None,
        };
        v.push(row);
    }
    match sender.send(v) {
        Ok(()) => {}
        Err(_) => {}
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
