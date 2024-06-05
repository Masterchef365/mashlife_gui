use anyhow::Result;
use eframe::egui::{self, DragValue, Response};
use egui::{Pos2, Rect, ScrollArea, Vec2};
use mashlife::{geometry::Coord, Handle, HashLife};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
type ZwoHasher = std::hash::BuildHasherDefault<zwohash::ZwoHasher>;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct MashlifeGui {
    grid_view: GridView,
    life: HashLife,
    world: Handle,

    frame_count: usize,

    time_step: usize,
    time_div: usize,

    view_center: Coord,
    //step_timing: Duration,
}

/// N large enough for big maps, but small enough for the machinery in MashLife to work... This
/// needs a more rigorous definition (or should just be 64)
const MAX_N: usize = 62;

impl Default for MashlifeGui {
    fn default() -> Self {
        let mut life = HashLife::new("B3/S23".parse().unwrap());
        let (rle, width) =
            mashlife::io::parse_rle(include_str!("builtin_patterns/clock.rle")).unwrap();
        let (input, view_center) = load_rle(&rle, width, &mut life).unwrap();

        let instance = Self {
            grid_view: GridView::new(),
            world: input,
            view_center,
            life,
            time_step: 0,
            //step_timing: Duration::ZERO,
            time_div: 1,
            frame_count: 0,
        };

        instance
    }
}

impl MashlifeGui {
    fn time_step(&mut self, time_step: usize) {
        //let time_start = Instant::now();

        let handle = self.life.result(self.world, time_step, (0, 0));
        self.world = self.life.expand(handle);

        // Automatic garbage collection
        let (result_bytes, parent_bytes, macrocells_bytes) = self.life.mem_usage();
        let total = result_bytes + parent_bytes + macrocells_bytes;

        //#[cfg(target_family = "wasm")]
        //let mem_limit = 1024usize.pow(2) * 500; // 500 MB
        //#[cfg(not(target_family = "wasm"))]
        let mem_limit = 1024usize.pow(3); // 1GB
                                          //let mem_limit = 1024usize.pow(3); // 1 GB

        if total > mem_limit {
            let (new_life, new_world) = self.life.gc(self.world);
            self.world = new_world;
            self.life = new_life;
        }

        //self.step_timing = time_start.elapsed();
    }
}

impl eframe::App for MashlifeGui {
    /*
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        ctx: &egui::Context,
        _frame: &eframe::Frame,
        _storage: Option<&dyn eframe::Storage>,
    ) {
        ctx.set_visuals(egui::Visuals::dark());
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        }
    }
    */

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update each frame
        if self.time_step != 0 {
            ctx.request_repaint();
        }

        if self.frame_count > self.time_div {
            self.time_step(self.time_step);
            self.frame_count = 0;
        }

        self.frame_count += 1;

        egui::TopBottomPanel::top("Menu bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                /*
                ui.menu_button("File", |ui| {
                    if ui.button("Load RLE from file").clicked() {}
                    if ui.button("Paste RLE from clipboard").clicked() {}

                    if ui.button("Save RLE to file").clicked() {}
                    if ui.button("Copy RLE to clipboard").clicked() {}
                });
                */

                ui.menu_button("Examples", |ui| {
                    egui::ScrollArea::new([false, true]).show(ui, |ui| {
                        ui.label("All credit to these patterns' creators at");
                        ui.hyperlink("https://conwaylife.com/wiki/");
                        ui.separator();
                        for &(name, rle) in BUILTIN_PATTERNS {
                            if ui.button(name).clicked() {
                                let mut life = HashLife::new("B3/S23".parse().unwrap());
                                let (rle, width) = mashlife::io::parse_rle(rle).unwrap();
                                let (input, view_center) =
                                    load_rle(&rle, width, &mut life).unwrap();
                                self.life = life;
                                self.world = input;
                                self.view_center = view_center;
                            }
                        }
                    });
                });
                ui.label("Time step: ");

                if ui.button("- -").clicked() {
                    if self.time_step == 1 {
                        self.time_step = 0;
                    } else {
                        self.time_step = 1
                            << (usize::BITS - self.time_step.leading_zeros())
                                .checked_sub(2)
                                .unwrap_or(0)
                    }
                }

                ui.add(DragValue::new(&mut self.time_step));

                if ui.button("++").clicked() {
                    self.time_step = 1 << (usize::BITS - self.time_step.leading_zeros())
                }

                ui.add(
                    DragValue::new(&mut self.time_div)
                        .prefix("/")
                        .clamp_range(1..=1000),
                );

                if ui.button("Step").clicked() {
                    self.time_step(1);
                }

                let (result_bytes, parent_bytes, macrocells_bytes) = self.life.mem_usage();
                ui.label(format!("Results: {}", format_mem_size(result_bytes)));
                ui.label(format!("Parents: {}", format_mem_size(parent_bytes)));
                ui.label(format!("Macrocells: {}", format_mem_size(macrocells_bytes)));
                ui.label(format!(
                    "Total: {}",
                    format_mem_size(result_bytes + parent_bytes + macrocells_bytes)
                ));
                //ui.label(format!("Step time: {}ms", self.step_timing.as_millis() as f32));
            });
        });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.collapsing("Macrocell n", |ui| {
                let n = self
                    .life
                    .macrocells()
                    .iter()
                    .map(|cell| cell.n)
                    .max()
                    .unwrap_or(0);
                let mut n_buckets = vec![0; n + 1];
                for cell in self.life.macrocells() {
                    n_buckets[cell.n] += 1;
                }

                ScrollArea::vertical().show(ui, |ui| {
                    for (n, bucket) in n_buckets.iter().enumerate() {
                        ui.label(format!("{n}: {bucket}"));
                    }
                });
            });

            ui.collapsing("Result t", |ui| {
                let t = self
                    .life
                    .results()
                    .map(|((t, _), _)| *t)
                    .max()
                    .unwrap_or(0);
                let mut t_buckets = vec![0; t + 1];

                for ((t, _), _) in self.life.results() {
                    t_buckets[*t] += 1;
                }

                ScrollArea::vertical().show(ui, |ui| {
                    for (n, bucket) in t_buckets.iter().enumerate() {
                        ui.label(format!("{n}: {bucket}"));
                    }
                });
            });

        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.grid_view
                .show(ui, &mut self.world, &mut self.life, self.view_center);
        });
    }
}

fn format_mem_size(size: usize) -> String {
    let mut s = String::new();

    let mag: usize = 1024;

    let sizes = [
        (mag.pow(0), "bytes"),
        (mag.pow(1), "KB"),
        (mag.pow(2), "MB"),
        (mag.pow(3), "GB"),
        (mag.pow(4), "TB"),
        (mag.pow(5), "PB"),
    ];

    for (measure, name) in sizes {
        if size >= measure - 1 {
            s = format!("{} {}", size / measure, name);
        } else {
            break;
        }
    }

    s
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
    queued_changes: HashSet<Coord, ZwoHasher>,
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
            scale: 1e-1,
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
    pub fn modify(&mut self, cursor_px: Pos2, view_size_px: Vec2) {
        let cursor_off_grid = self.calc_cursor_grid(cursor_px, view_size_px);

        let cursor_pos_grid = self.center + cursor_off_grid;

        let cursor_off_grid_int = (
            cursor_pos_grid.x.round() as i64,
            cursor_pos_grid.y.round() as i64,
        );

        self.queued_changes.insert(cursor_off_grid_int);
    }

    /// The current view rect, in grid space
    pub fn viewbox_grid(&self, view_size_px: Vec2) -> Rect {
        let view_size_grid = view_size_px / self.scale;
        Rect::from_center_size(self.center, view_size_grid)
    }

    /// Return the rectangles of the pixels which are in view
    pub fn view_rects(&self, view_size_px: Vec2) -> impl Iterator<Item = Rect> + '_ {
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
                    tile_size_px,
                )
            })
        })
    }

    fn update_life(&mut self, life: &mut HashLife, mut node: Handle) -> Handle {
        // Apply pending changes
        for (x, y) in self.queued_changes.drain() {
            let coord = (x + (1 << MAX_N - 1), y + (1 << MAX_N - 1));
            let value = !life.read(node, coord);
            node = life.modify(node, coord, value, MAX_N);
        }

        node
    }

    fn render_life(
        &mut self,
        view_center: Coord,
        life: &mut HashLife,
        mut node: Handle,
        grid_size: Vec2,
    ) {
        // Render result
        let min_n = self.min_n();
        self.grid.clear();

        let rect = self.viewbox_grid(grid_size);

        let mut set_grid = |(x, y)| {
            let _ = self.grid.insert((x as _, y as _));
        };

        let (left, top) = view_center;
        let rect = (
            (
                rect.min.x.floor() as i64 + left,
                rect.min.y.floor() as i64 + top,
            ),
            (
                rect.max.x.ceil() as i64 + left,
                rect.max.y.ceil() as i64 + top,
            ),
        );

        life.resolve((0, 0), &mut set_grid, min_n, rect, node);
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        node: &mut Handle,
        life: &mut HashLife,
        view_center: Coord,
    ) -> Response {
        let area = ui.available_size();
        let (display_rect, response) = ui.allocate_exact_size(area, egui::Sense::click_and_drag());

        // Clip outside the draw space
        let mut ui = ui.child_ui(display_rect, egui::Layout::default());
        ui.set_clip_rect(display_rect);

        // Dragging
        if response.dragged_by(egui::PointerButton::Secondary)
            || (response.dragged_by(egui::PointerButton::Primary)
                && ui.input(|r| r.modifiers.shift_only()))
        {
            self.drag(response.drag_delta());
        }

        // Zooming
        if let Some(hover_pos) = response.hover_pos() {
            let cursor_relative = hover_pos - display_rect.min.to_vec2();

            self.zoom(
                ui.input(|r| r.smooth_scroll_delta.y * 0.001),
                cursor_relative,
                display_rect.size(),
            );

            if response.clicked() {
                self.modify(cursor_relative, display_rect.size());
            }

            /*if response.dragged_by(egui::PointerButton::Primary) {
            self.modify(cursor_relative, display_rect.size());
            }*/
        }

        // Drawing
        if ui.is_rect_visible(display_rect) {
            // Background
            ui.painter()
                .rect(display_rect, 0., egui::Color32::BLACK, egui::Stroke::NONE);

            //dbg!(self.scale, self.center, self.grid.len());
            for tile in self.view_rects(area) {
                ui.painter().rect(
                    tile.translate(display_rect.min.to_vec2()),
                    0.,
                    egui::Color32::WHITE,
                    egui::Stroke::NONE,
                );
            }
        }

        *node = self.update_life(life, *node);
        self.render_life(view_center, life, *node, area);

        response
    }
}

fn load_rle(rle: &[bool], rle_width: usize, life: &mut HashLife) -> Result<(Handle, Coord)> {
    let rle_height = rle.len() / rle_width;

    let n = MAX_N;

    let half_width = 1 << n - 1;

    let insert_tl = (
        half_width - rle_width as i64 / 2,
        half_width - rle_height as i64 / 2,
    );

    let input_cell = life.insert_array(&rle, rle_width, insert_tl, n as _);

    let view_center = (half_width, half_width);

    Ok((input_cell, view_center))
}

macro_rules! builtin_pattern {
    ($path:expr) => {
        ($path, include_str!(concat!("builtin_patterns/", $path)))
    };
}

const BUILTIN_PATTERNS: &[(&str, &str)] = &[
    builtin_pattern!("10cellinfinitegrowth.rle"),
    builtin_pattern!("2005-07-23-switch-breeder.rle"),
    builtin_pattern!("2011-01-10-HH-c5-grey-part.rle"),
    builtin_pattern!("2011-01-10-HH-c5-greyship.rle"),
    builtin_pattern!("2011-08-26-c7-extensible.rle"),
    builtin_pattern!("52513m.rle"),
    builtin_pattern!("acorn.rle"),
    builtin_pattern!("anura.rle"),
    builtin_pattern!("broken-lines.rle"),
    builtin_pattern!("catacryst.rle"),
    builtin_pattern!("clock.rle"),
    builtin_pattern!("gotts-dots.rle"),
    builtin_pattern!("hashlife-oddity2.rle"),
    builtin_pattern!("hivenudger2.rle"),
    builtin_pattern!("jagged.rle"),
    builtin_pattern!("logarithmic-width.rle"),
    builtin_pattern!("metapixel-galaxy.rle"),
    builtin_pattern!("OTCAmetapixel.rle"),
    builtin_pattern!("richsp16.rle"),
    builtin_pattern!("smallp120hwssgun.rle"),
    builtin_pattern!("sprayer.rle"),
];
