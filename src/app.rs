use eframe::{egui, epi};
use egui::{Pos2, Rect, Vec2};
use std::collections::{HashSet, HashMap};
use mashlife::{HashLife, Handle, Modification};
use std::path::Path;
use anyhow::{Result, Context};
type ZwoHasher = std::hash::BuildHasherDefault<zwohash::ZwoHasher>;

const GRID_SIZE: Vec2 = Vec2::new(720., 480.);

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct MashlifeGui {
    grid_view: GridView,
    life: HashLife,
    input: Handle,
    time_step: usize,
    view_center: Coord,
}

const DEFAULT_N: usize = 60;

impl Default for MashlifeGui {
    fn default() -> Self {
        let mut life = HashLife::new("B3/S23".parse().unwrap());
        let (input, view_center) = load_rle("mashlife/life/52513m.rle", &mut life).unwrap();

        let mut instance = Self { 
            grid_view: GridView::new(),
            input,
            view_center,
            life,
            time_step: 0,
        };

        instance.render_time_step(instance.time_step);

        instance
    }
}

impl MashlifeGui {
    fn render_time_step(&mut self, time_step: usize) {
        // TODO: Keep the grid at a specific size N by taking the time-stepped and modified grid
        // (smaller than input) and surrounding it with zeroes and making a new cell.

        // Apply pending changes
        for ((x, y), modif) in self.grid_view.queued_changes.drain() {
            let coord = (x + (1 << DEFAULT_N - 1), y + (1 << DEFAULT_N - 1));
            self.input = self.life.modify(self.input, coord, modif, DEFAULT_N);
        }

        // Calculate result
        let handle = self.life.result(self.input, time_step, (0, 0), 0);

        // Render result
        let min_n = self.grid_view.min_n();
        self.grid_view.grid.clear();

        let rect = self.grid_view.viewbox_grid(GRID_SIZE);

        let mut set_grid = |(x, y)| { let _ = self.grid_view.grid.insert((x as _, y as _)); };

        let (left, top) = self.view_center;
        let rect = (
            (rect.min.x.floor() as i64 + left, rect.min.y.floor() as i64 + top),
            (rect.max.x.ceil() as i64 + left, rect.max.y.ceil() as i64 + top),
        );

        self.life.resolve((0, 0), &mut set_grid, min_n, rect, handle);
    }
}

impl epi::App for MashlifeGui {
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
        self.render_time_step(self.time_step);

        //self.time_step += 1;

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
            ui.add(grid_square(&mut self.grid_view, GRID_SIZE));
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
    if response.dragged_by(egui::PointerButton::Secondary) {
        grid_view.drag(response.drag_delta());
    }

    // Zooming
    if let Some(hover_pos) = response.hover_pos() {
        let cursor_relative = hover_pos - display_rect.min.to_vec2();

        grid_view.zoom(
            ui.input().scroll_delta.y * 0.001,
            cursor_relative,
            display_rect.size(),
        );

        if response.clicked() {
            grid_view.modify(cursor_relative, display_rect.size(), Modification::Toggle);
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            // TODO: "pick" a single pixel instead of `Modification`, then be able to draw a line
            // of pixels?
            grid_view.modify(cursor_relative, display_rect.size(), Modification::Alive);
        }
    }

    // Drawing
    if ui.is_rect_visible(display_rect) {
        // Background
        ui.painter()
            .rect(display_rect, 0., egui::Color32::BLACK, egui::Stroke::none());

        //dbg!(grid_view.scale, grid_view.center, grid_view.grid.len());
        for tile in grid_view.view(scale) {
            ui.painter().rect(
                tile.translate(display_rect.min.to_vec2()),
                0.,
                egui::Color32::WHITE,
                egui::Stroke::none(),
            );
        }
    }

    response
}

type Grid = HashSet<(i32, i32), ZwoHasher>;

// TODO: Use a rect, and scroll with respect to the cursor.
pub struct GridView {
    /// The center of the view, in grid units
    center: Pos2,
    /// Pixels per tile
    scale: f32,
    /// Grid cells which are on, and their counts
    grid: Grid,
    /// Changes to be applied to the game when ready 
    queued_changes: HashMap<Coord, Modification, ZwoHasher>,
}

impl GridView {
    pub fn new() -> Self {
        Self::from_grid(Grid::default())
    }

    pub fn min_n(&self) -> usize {
        (1. / self.scale).log2() as usize
    }

    /// Create a new instance from a grid
    pub fn from_grid(grid: Grid) -> Self {
        Self {
            scale: 20.,
            center: Pos2::ZERO,
            grid,
            queued_changes: Default::default(),
        }
    }

    /// Handle a drag action
    pub fn drag(&mut self, delta: Vec2) {
        self.center -= delta / self.scale;
    }

    fn calc_cursor_grid(&self, cursor_px: Pos2, view_size_px: Vec2) -> Vec2 {
        let view_center_px = view_size_px / 2.;
        let cursor_off_px = cursor_px - view_center_px;
        let cursor_off_grid = cursor_off_px.to_vec2() / self.scale;
        cursor_off_grid
    }

    /// Handle a zoom action
    pub fn zoom(&mut self, delta: f32, cursor_px: Pos2, view_size_px: Vec2) {
        self.scale += delta * self.scale;
        self.center += self.calc_cursor_grid(cursor_px, view_size_px) * delta;
    }

    /// Handle a click
    pub fn modify(&mut self, cursor_px: Pos2, view_size_px: Vec2, modif: Modification) {
        let cursor_off_grid = self.calc_cursor_grid(cursor_px, view_size_px);

        let cursor_pos_grid = self.center + cursor_off_grid;

        let cursor_off_grid_int = (
            cursor_pos_grid.x.round() as i64,
            cursor_pos_grid.y.round() as i64,
        );

        self.queued_changes.insert(cursor_off_grid_int, modif);
    }

    /// The current view rect, in grid space
    pub fn viewbox_grid(&self, view_size_px: Vec2) -> Rect {
        let view_size_grid = view_size_px / self.scale;
        Rect::from_center_size(self.center, view_size_grid)
    }

    /// Return the rectangles of the pixels which are in view
    pub fn view(&self, view_size_px: Vec2) -> impl Iterator<Item = Rect> + '_ {
        let view_center_px = view_size_px / 2.;

        let view_rect_grid = self.viewbox_grid(view_size_px);

        let cell_scale_grid = (1 << self.min_n()) as f32;
        let cell_scale_grid_px = cell_scale_grid * self.scale;

        let tile_size_grid = Vec2::splat(cell_scale_grid);
        let tile_size_px = Vec2::splat(cell_scale_grid_px);

        self.grid.iter().filter_map(move |&(x, y)| {
            let pos_grid = Pos2::new(x as f32, y as f32);
            let rect = Rect::from_center_size(pos_grid, tile_size_grid);

            view_rect_grid.intersects(rect).then(move || {
                Rect::from_center_size(
                    ((pos_grid - self.center) * self.scale + view_center_px).to_pos2(),
                    tile_size_px
                )
            })
        })
    }
}

use mashlife::Coord;

fn load_rle(_path: impl AsRef<Path>, life: &mut HashLife) -> Result<(Handle, Coord)> {
    // Load RLE
    //let (rle, rle_width) =
        //mashlife::io::load_rle(path).context("Failed to load RLE file")?;
    let (rle, rle_width) =
        //mashlife::io::parse_rle(include_str!("../../mashlife/life/metapixel-galaxy.rle")).context("Failed to load RLE file")?;
        //mashlife::io::parse_rle(include_str!("../../mashlife/life/clock.rle")).context("Failed to load RLE file")?;
        mashlife::io::parse_rle(include_str!("../../mashlife/life/52513m.rle")).context("Failed to load RLE file")?;

    let rle_height = rle.len() / rle_width;

    //let max_rle_dim = rle_height.max(rle_width);

    //eprintln!("REMOVE THESE DEFAULTS!!");
    //let expected_steps = 100 as u64 + 12_000;
    let n = DEFAULT_N;
        //highest_pow_2(max_rle_dim as _)
        //.max(highest_pow_2(expected_steps) + 2);

    let half_width = 1 << n - 1;
    let quarter_width = 1 << n - 2;

    let insert_tl = (
        half_width - rle_width as i64 / 2,
        half_width - rle_height as i64 / 2
    );

    //let start = std::time::Instant::now();
    let input_cell = life.insert_array(&rle, rle_width, insert_tl, n as _);
    /*println!(
        "Input insertion took {}ms",
        start.elapsed().as_secs_f32() * 1e3
    );*/

    let view_center = (
        quarter_width,
        quarter_width
    );

    Ok((input_cell, view_center))
}

/*fn highest_pow_2(v: u64) -> u32 {
    8 * std::mem::size_of_val(&v) as u32 - v.leading_zeros()
}*/
