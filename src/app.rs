use eframe::{egui, epi};
use egui::{Pos2, Rect, Vec2};
use std::collections::HashSet;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    grid_view: GridView,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut grid = Grid::new();

        for x in -100i32..100 {
            for y in -100i32..100 {
                if x.count_ones() < y.count_ones() {
                    grid.insert((x, y));
                }
            }
        }

        let grid_view = GridView::from_grid(grid);

        Self { grid_view }
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
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.add(grid_square(&mut self.grid_view, Vec2::splat(400.)));

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

pub fn grid_square(grid_view: &mut GridView, scale: Vec2) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| grid_square_ui(ui, grid_view, scale)
}

pub fn grid_square_ui(ui: &mut egui::Ui, grid_view: &mut GridView, scale: Vec2) -> egui::Response {
    grid_view.center.x = 8.0;
    grid_view.center.y = 8.0;
    grid_view.scale = dbg!(50. - grid_view.t.elapsed().as_secs_f32().sqrt() * 10.1);
    let (display_rect, response) = ui.allocate_exact_size(scale, egui::Sense::hover());

    let mut ui = ui.child_ui(display_rect, egui::Layout::default());
    ui.set_clip_rect(display_rect);

    if ui.is_rect_visible(display_rect) {
        // Background
        ui.painter()
            .rect(display_rect, 0., egui::Color32::BLACK, egui::Stroke::none());

        for tile in grid_view.view(scale) {
            dbg!(tile);
            ui.painter()
                .rect(tile, 0., egui::Color32::WHITE, egui::Stroke::none());
        }
        eprintln!();
    }

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    response
}

type Grid = HashSet<(i32, i32)>;

pub struct GridView {
    /// The center of the view, in grid units
    center: Pos2,
    /// Pixels per tile
    scale: f32,
    /// Grid cells which are on
    grid: Grid,
    t: std::time::Instant,
}

impl GridView {
    pub fn new() -> Self {
        Self::from_grid(Grid::new())
    }

    pub fn from_grid(grid: Grid) -> Self {
        Self {
            scale: 50.,
            center: Pos2::ZERO,
            grid,
            t: std::time::Instant::now(),
        }
    }

    pub fn click(&mut self, pos: Pos2) {
        todo!()
    }

    pub fn view(&self, view_size_px: Vec2) -> impl Iterator<Item = Rect> + '_ {
        let view_center_px = view_size_px / 2.;
        let view_size_grid = view_size_px / self.scale;

        let view_rect_grid = Rect::from_center_size(self.center, view_size_grid);

        self.grid.iter().filter_map(move |&(x, y)| {
            let pos_grid = Pos2::new(x as f32, y as f32);
            let rect = Rect::from_min_size(pos_grid, Vec2::splat(1.));
            view_rect_grid.intersects(rect).then(move || {
                Rect::from_min_size(
                    ((pos_grid - self.center) * self.scale + view_center_px).to_pos2(),
                    Vec2::splat(self.scale),
                )
            })
        })
    }
}
