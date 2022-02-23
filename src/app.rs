use eframe::{egui, epi};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[cfg_attr(feature = "persistence", serde(skip))]
    value: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        ctx.set_visuals(egui::Visuals::dark());
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let Self { label, value } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            let pixels = [
                true, false, true, false, true, false, false, false, true, true, true, false, true,
                false, false, false,
            ];

            ui.add(grid_square(&pixels, 4));

            ui.heading("eframe template");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });
    }
}
pub fn grid_square(pixels: &[bool], width: usize) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| grid_square_ui(ui, pixels, width)
}

pub fn grid_square_ui(ui: &mut egui::Ui, pixels: &[bool], width: usize) -> egui::Response {
    // Widget code can be broken up in four steps:
    //  1. Decide a size for the widget
    //  2. Allocate space for it
    //  3. Handle interactions with the widget (if any)
    //  4. Paint the widget

    let display_width = 400.0;

    // 1. Deciding widget size:
    // You can query the `ui` how much space is available,
    // but in this example we have a fixed size widget based on the height of a standard button:
    let desired_size = egui::vec2(display_width, display_width); //ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);

    // 2. Allocating space:
    // This is where we get a region of the screen assigned.
    // We also tell the Ui to sense clicks in the allocated region.
    let (display_rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    // 4. Paint!
    // Make sure we need to paint:
    if ui.is_rect_visible(display_rect) {
        //let visuals = ui.style().interact(&response);
        ui.painter()
            .rect(display_rect, 0., egui::Color32::BLACK, egui::Stroke::none());
        let cell_width = display_width as f32 / width as f32;
        let cell_size = egui::vec2(cell_width, cell_width);
        for (row_idx, row) in pixels.chunks_exact(width).enumerate() {
            for (col_idx, elem) in row.iter().enumerate() {
                if *elem {
                    let pos = egui::pos2(
                        row_idx as f32 * cell_width,
                        col_idx as f32 * cell_width,
                    ) + display_rect.left_top().to_vec2();
                    let rect = egui::Rect::from_min_size(pos, cell_size);
                    ui.painter()
                        .rect(rect, 0., egui::Color32::WHITE, egui::Stroke::none());
                }
            }
        }
    }

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    response
}
