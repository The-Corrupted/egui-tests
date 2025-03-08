use crossbeam_channel::unbounded;
use eframe::egui;

// START PREPROCESSOR PASTE

#[derive(Default)]
enum AnimationState {
    Waiting,
    #[default]
    Animating,
    Done,
}

#[derive(Default)]
struct RowData {
    version: String,
    path: String,
}

#[derive(Default)]
struct AnimatedRow {
    data: RowData,
    progress: f32,
    start_x: Option<f32>,
    start_time: Option<f64>,
    elapsed: f32,
    animation_time: f32,
    state: AnimationState,
    delay: f32,
}

#[derive(Default)]
struct AnimatedRowList {
    rows: Vec<AnimatedRow>,
    row_shapes: Vec<egui::Shape>,
    row_height: f32,
}

impl RowData {
    fn new(version: String, path: String) -> Self {
        Self { version, path }
    }
}

impl AnimatedRow {
    fn new(row_data: RowData, duration: f32, delay: f32) -> Self {
        let state = if delay == 0.0 {
            AnimationState::Animating
        } else {
            AnimationState::Waiting
        };
        Self {
            data: row_data,
            start_x: None,
            start_time: None,
            progress: 0.0,
            elapsed: 0.0,
            animation_time: duration,
            state,
            delay,
        }
    }

    pub fn update(&mut self, time: f64) -> bool {
        match self.state {
            AnimationState::Waiting => {
                if let Some(start_time) = self.start_time {
                    self.elapsed = (time - start_time) as f32;
                    if self.elapsed >= self.delay {
                        self.state = AnimationState::Animating;
                        self.elapsed = 0.0;
                        self.start_time = Some(time);
                        return true;
                    }
                    return false;
                } else {
                    self.start_time = Some(time);
                    return false;
                }
            }
            AnimationState::Animating => {
                if let Some(start_time) = self.start_time {
                    self.elapsed = (time - start_time) as f32;
                    self.progress = egui::emath::easing::quadratic_out(
                        (self.elapsed / self.animation_time).min(1.0),
                    );
                    if self.progress == 1.0 {
                        self.state = AnimationState::Done;
                        return false;
                    }
                    return true;
                } else {
                    // No duration so we start by animating
                    self.start_time = Some(time);
                    return true;
                }
            }
            AnimationState::Done => false,
        }
    }
}

impl AnimatedRowList {
    pub fn new(rows: Vec<RowData>, animation_duration: f32, stagger_delay: f32) -> Self {
        let len = rows.len();
        let animated_rows = rows
            .into_iter()
            .enumerate()
            .map(|(i, data)| AnimatedRow::new(data, animation_duration, i as f32 * stagger_delay))
            .collect();

        let row_height = 60.0;

        Self {
            rows: animated_rows,
            row_shapes: Vec::with_capacity(len * 2),
            row_height,
        }
    }

    pub fn show(&mut self, resized: bool, ui: &mut egui::Ui) -> bool {
        let mut needs_redraw = false;
        ui.vertical(|ui| {
            let time = ui.input(|i| i.time);
            for row in &mut self.rows {
                if row.start_x.is_none() || resized {
                    row.start_x = Some(ui.max_rect().width());
                }

                needs_redraw |= row.update(time);

                let (id, rect) =
                    ui.allocate_space(egui::Vec2::new(ui.available_width(), self.row_height));

                let response = ui.interact(rect, id, egui::Sense::click());

                if response.clicked_by(egui::PointerButton::Primary) {
                    println!("Row {} clicked", id.short_debug_format());
                }

                let start_x = row.start_x.unwrap();
                let target_x = rect.left();
                let x_offset = start_x + (target_x - start_x) * row.progress;
                let animated_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(x_offset, rect.top()),
                    egui::Vec2::new(rect.width(), rect.height()),
                );

                let alpha = (255.0 * row.progress) as u8;
                self.row_shapes
                    .push(egui::Shape::Rect(egui::epaint::RectShape {
                        rect: animated_rect,
                        corner_radius: egui::epaint::CornerRadius::from(100),
                        fill: egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                        stroke: egui::Stroke::new(0.0, egui::Color32::TRANSPARENT),
                        stroke_kind: egui::StrokeKind::Inside,
                        round_to_pixels: None,
                        blur_width: 0.0,
                        brush: None,
                    }));

                let text_galley = ui.painter().layout_no_wrap(
                    row.data.version.clone(),
                    egui::FontId::new(20.0, egui::FontFamily::Proportional),
                    egui::Color32::from_rgba_premultiplied(0, 0, 0, alpha),
                );

                let mut text_pos = animated_rect.center();
                text_pos.x -= animated_rect.width() * 0.02;
                text_pos.y -= animated_rect.height() * 0.2;

                self.row_shapes
                    .push(egui::Shape::Text(egui::epaint::TextShape {
                        pos: text_pos,
                        galley: text_galley,
                        override_text_color: None,
                        angle: 0.0,
                        fallback_color: egui::Color32::BLACK,
                        underline: egui::Stroke::NONE,
                        opacity_factor: row.progress,
                    }));
            }
        });
        ui.painter().extend(std::mem::take(&mut self.row_shapes));
        needs_redraw
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
    last_width: Option<f32>,
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
            last_width: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // We need to retrieve some rows
        let mut resized = false;
        let mut needs_repaint = false;
        let screen_width = ctx.input(|i| i.screen_rect().width());
        if let Some(last_width) = self.last_width {
            if screen_width != last_width {
                self.last_width = Some(screen_width);
                resized = true;
            }
        } else {
            self.last_width = Some(screen_width);
            resized = true;
        }
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
                    needs_repaint = row_list.show(resized, ui);
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
                    self.rows = Some(AnimatedRowList::new(t, 1.0, 0.1));
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

        if needs_repaint {
            ctx.request_repaint();
        }
    }
}

fn delayed_fetch_operation(page: u32, sender: &crossbeam_channel::Sender<Vec<RowData>>) {
    let count = 5;
    for _ in (0..count).rev() {
        std::thread::sleep(std::time::Duration::new(1, 0));
    }

    let mut v: Vec<RowData> = Vec::new();
    for x in 0..100 {
        let row: RowData = RowData {
            version: String::from(format!("ProtonGE-{}", x)),
            path: String::from(format!("some/path/{}", x)),
        };
        v.push(row);
    }
    match sender.send(v) {
        Ok(()) => {}
        Err(_) => {}
    }
}

fn main() {
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
