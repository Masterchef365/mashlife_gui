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

        let k: i32 = 4000;
        for x in -k..=k {
            for y in -k..=k {
                if (x.abs() * 3402 + y.abs() * 4281).count_ones() < 5 {
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

            ui.heading("eframe template");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);

            // The central panel the region left after adding TopPanel's and SidePanel's
            //let t = std::time::Instant::now();
            ui.add(grid_square(&mut self.grid_view, Vec2::splat(400.)));
            //dbg!(t.elapsed().as_secs_f32());
        });
    }
}

pub fn grid_square(grid_view: &mut GridView, scale: Vec2) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| grid_square_ui(ui, grid_view, scale)
}

pub fn grid_square_ui(ui: &mut egui::Ui, grid_view: &mut GridView, scale: Vec2) -> egui::Response {
    let (display_rect, response) = ui.allocate_exact_size(scale, egui::Sense::click_and_drag());

    // Clip outside the draw space
    let mut ui = ui.child_ui(display_rect, egui::Layout::default());
    ui.set_clip_rect(display_rect);

    // Dragging
    if response.dragged_by(egui::PointerButton::Primary) {
        grid_view.drag(response.drag_delta());
    }

    // Zooming
    if let Some(hover_pos) = response.hover_pos() {
        grid_view.zoom(
            ui.input().scroll_delta.y * 0.001,
            hover_pos,
            display_rect.size(),
        );

        if response.clicked() {
            grid_view.click(
                hover_pos,
                display_rect.size(),
            );
        }
    }

    // Drawing
    if ui.is_rect_visible(display_rect) {
        // Background
        ui.painter()
            .rect(display_rect, 0., egui::Color32::BLACK, egui::Stroke::none());

        for tile in grid_view.view(scale) {
            ui.painter()
                .rect(tile, 0., egui::Color32::WHITE, egui::Stroke::none());
        }
        eprintln!();
    }

    if let Some(hover_pos) = response.hover_pos() {
        ui.painter()
            .rect(Rect::from_min_size(hover_pos, Vec2::splat(10.)), 0., egui::Color32::RED, egui::Stroke::none());
    }

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    response
}

type Grid = HashSet<(i32, i32)>;

// TODO: Use a rect, and scroll with respect to the cursor.
pub struct GridView {
    /// The center of the view, in grid units
    center: Pos2,
    /// Pixels per tile
    scale: f32,
    /// Grid cells which are on
    grid: Grid,
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
        }
    }

    pub fn drag(&mut self, delta: Vec2) {
        self.center -= delta / self.scale;
    }

    pub fn zoom(&mut self, delta: f32, cursor_px: Pos2, view_size_px: Vec2) {
        self.scale += delta * self.scale;

        let view_center_px = view_size_px / 2.;
        let cursor_off_px = cursor_px - view_center_px;
        let cursor_off_grid = cursor_off_px.to_vec2() / self.scale;

        self.center += cursor_off_grid * delta;

    }

    pub fn click(&mut self, cursor_px: Pos2, view_size_px: Vec2) {
        let view_center_px = view_size_px / 2.;
        let cursor_off_px = cursor_px - view_center_px;
        let cursor_off_grid = cursor_off_px.to_vec2() / self.scale;
        let cursor_pos_grid = self.center + cursor_off_grid;

        let cursor_off_grid_int = (cursor_pos_grid.x.round() as i32, cursor_off_grid.y.round() as i32);

        if self.grid.get(&cursor_off_grid_int).is_some() {
            self.grid.remove(&cursor_off_grid_int);
        } else {
            self.grid.insert(cursor_off_grid_int);
        }
    }

    pub fn view(&self, view_size_px: Vec2) -> impl Iterator<Item = Rect> + '_ {
        let view_center_px = view_size_px / 2.;
        let view_size_grid = view_size_px / self.scale;

        let view_rect_grid = Rect::from_center_size(self.center, view_size_grid);

        self.grid.iter().filter_map(move |&(x, y)| {
            let pos_grid = Pos2::new(x as f32, y as f32);
            let rect = Rect::from_center_size(pos_grid, Vec2::splat(1.));
            view_rect_grid.intersects(rect).then(move || {
                Rect::from_center_size(
                    ((pos_grid - self.center) * self.scale + view_center_px).to_pos2(),
                    Vec2::splat(self.scale),
                )
            })
        })
    }
}
