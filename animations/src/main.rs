use eframe::egui;

fn main() {
    start_puffin_server();
    let options = set_native_options();

    let result = eframe::run_native(
        "Animation Widget",
        options,
        Box::new(|cc| Ok(Box::new(AnimationApp::new(cc)))),
    );

    match result {
        Ok(()) => (),
        Err(e) => {
            println!("Failed to exit app properly: {}", e);
        }
    }
}

pub fn set_native_options() -> eframe::NativeOptions {
    let mut options = eframe::NativeOptions::default();
    options.centered = true;
    options.vsync = true;
    options.renderer = eframe::Renderer::Wgpu;
    options
}

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
    start_time: f64,
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
    fn new(row_data: RowData, start_time: f64, duration: f32, delay: f32) -> Self {
        let state = if delay == 0.0 {
            AnimationState::Animating
        } else {
            AnimationState::Waiting
        };
        Self {
            data: row_data,
            start_x: None,
            start_time,
            progress: 0.0,
            elapsed: 0.0,
            animation_time: duration,
            state,
            delay,
        }
    }

    pub fn update(&mut self, time: f64) -> bool {
        puffin::profile_scope!("AnimatedRow::update");
        match self.state {
            AnimationState::Waiting => {
                self.elapsed = (time - self.start_time) as f32;
                if self.elapsed >= self.delay {
                    self.state = AnimationState::Animating;
                    self.elapsed = 0.0;
                    self.start_time = time;
                    return true;
                }
                false
            }
            AnimationState::Animating => {
                self.elapsed = (time - self.start_time) as f32;
                self.progress = egui::emath::easing::quadratic_out(
                    (self.elapsed / self.animation_time).min(1.0),
                );
                if self.progress == 1.0 {
                    self.state = AnimationState::Done;
                    return false;
                }
                true
            }
            AnimationState::Done => false,
        }
    }
}

impl AnimatedRowList {
    pub fn new(
        rows: Vec<RowData>,
        start_time: f64,
        animation_duration: f32,
        stagger_delay: f32,
    ) -> Self {
        let len = rows.len();
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

        let row_height = 60.0;

        Self {
            rows: animated_rows,
            row_shapes: Vec::with_capacity(len * 2),
            row_height,
        }
    }

    pub fn show(&mut self, resized: bool, ui: &mut egui::Ui) -> bool {
        puffin::profile_scope!("AnimatedRowList::show");
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

#[derive(Default)]
struct AnimationApp {
    row_list: AnimatedRowList,
    last_width: Option<f32>,
}

impl AnimationApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut it = Self::default();
        let mut rows = Vec::with_capacity(100);
        for x in 0..=100 {
            rows.push(RowData::new(
                format!("GE-Proton9-{}", x),
                format!("/some/path/{}", x),
            ));
        }
        it.row_list = AnimatedRowList::new(rows, cc.egui_ctx.input(|i| i.time), 1.0, 0.1);
        it
    }
}

impl eframe::App for AnimationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame();
        puffin::profile_scope!("AnimationApp::update");
        let screen_width = ctx.input(|i| i.screen_rect.width());
        let mut need_redraw = false;
        let mut resized = false;
        if self.last_width.is_none() || self.last_width.unwrap() != screen_width {
            resized = true;
            self.last_width = Some(screen_width);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                need_redraw |= self.row_list.show(resized, ui);
            })
        });

        if need_redraw {
            ctx.request_repaint();
        }

        let frame_time = ctx.input(|i| i.stable_dt);
        let fps = 1.0 / frame_time;
        println!("FPS: {}", fps);
    }
}

fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            // log::info!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            // log::error!("Failed to start puffin server: {err}");
        }
    };
}
