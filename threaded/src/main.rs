use crossbeam_channel::unbounded;
use eframe::egui;

pub fn set_native_options() -> eframe::NativeOptions {
    let mut options = eframe::NativeOptions::default();
    options.centered = true;
    options.vsync = true;
    options.renderer = eframe::Renderer::Wgpu;
    options
}

struct Row {
    version: String,
    path: String,
}

struct App {
    sql_sender: crossbeam_channel::Sender<Vec<Row>>,
    sql_receiver: crossbeam_channel::Receiver<Vec<Row>>,
    fetch_thread: Option<std::thread::JoinHandle<()>>,
    current_page: u32,
    info_retrieved: bool,
    rows: Option<Vec<Row>>,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (s, r) = unbounded();
        Self {
            sql_sender: s,
            sql_receiver: r,
            fetch_thread: None,
            info_retrieved: false,
            current_page: 1,
            rows: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // We need to retrieve some rows
        if !self.info_retrieved && self.fetch_thread.is_none() {
            let func = move |cloned_sender: crossbeam_channel::Sender<Vec<Row>>, page: u32| {
                delayed_fetch_operation(page, &cloned_sender);
            };
            let cloned_sender = self.sql_sender.clone();
            let page = self.current_page;
            let handle = std::thread::spawn(move || func(cloned_sender, page));
            self.fetch_thread = Some(handle);
        }

        // If we find some rows, display them. If not, say we're fetching them
        if let Some(rows) = &self.rows {
            egui::CentralPanel::default().show(ctx, |ui| {
                for row in rows {
                    ui.label(format!("Row Version: {}", row.version));
                }
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Fetching rows");
            });
        }

        // Check if the sender has finished
        if !self.info_retrieved {
            match self.sql_receiver.try_recv() {
                Ok(t) => {
                    self.rows = Some(t);
                    self.info_retrieved = true;
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

fn delayed_fetch_operation(page: u32, sender: &crossbeam_channel::Sender<Vec<Row>>) {
    let count = 5;
    for _ in (0..count).rev() {
        std::thread::sleep(std::time::Duration::new(1, 0));
    }

    let mut v: Vec<Row> = Vec::new();
    for x in 0..count {
        let row: Row = Row {
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
