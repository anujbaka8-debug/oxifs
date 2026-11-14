use crate::disk::{self, DiskInfo};
use crate::fs_ops::{self, FileEntry, FsMsg, format_gb, format_mb, human_size};
use egui::*;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::path::PathBuf;
use std::sync::mpsc;

// ── Catppuccin Mocha Palette ──────────────────────────────────────────────────
const BASE:     Color32 = Color32::from_rgb(30,  30,  46);
const MANTLE:   Color32 = Color32::from_rgb(24,  24,  37);
const CRUST:    Color32 = Color32::from_rgb(17,  17,  27);
const SURFACE0: Color32 = Color32::from_rgb(49,  50,  68);
const SURFACE1: Color32 = Color32::from_rgb(69,  71,  90);
const TEXT:     Color32 = Color32::from_rgb(205, 214, 244);
const SUBTEXT:  Color32 = Color32::from_rgb(166, 173, 200);
const OVERLAY:  Color32 = Color32::from_rgb(108, 112, 134);
const PINK:     Color32 = Color32::from_rgb(245, 194, 231);
const BLUE:     Color32 = Color32::from_rgb(137, 180, 250);
const GREEN:    Color32 = Color32::from_rgb(166, 227, 161);
const YELLOW:   Color32 = Color32::from_rgb(249, 226, 175);
const PEACH:    Color32 = Color32::from_rgb(250, 179, 135);
const RED:      Color32 = Color32::from_rgb(243, 139, 168);
const MAUVE:    Color32 = Color32::from_rgb(203, 166, 247);
const TEAL:     Color32 = Color32::from_rgb(148, 226, 213);
const SAPPHIRE: Color32 = Color32::from_rgb(116, 199, 236);
const LAVENDER: Color32 = Color32::from_rgb(180, 190, 254);

const SLICE_COLORS: [Color32; 9] = [
    PEACH, MAUVE, BLUE, TEAL, PINK, YELLOW, GREEN, SAPPHIRE, LAVENDER,
];

// ── App State ─────────────────────────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum Clip {
    None,
    Copy(PathBuf),
    Cut(PathBuf),
}

#[derive(Clone, PartialEq)]
enum InputMode {
    Rename,
    Mkdir,
}

struct InputState {
    mode: InputMode,
    value: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    Storage,
    Preview,
}

pub struct OxifsApp {
    cwd: PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    scroll_to: bool,
    show_hidden: bool,

    active_tab: RightTab,

    clip: Clip,
    status: String,
    loading: bool,

    disks: Vec<DiskInfo>,

    tx: mpsc::Sender<FsMsg>,
    rx: mpsc::Receiver<FsMsg>,

    search: String,
    search_active: bool,

    confirm_delete: bool,
    input: Option<InputState>,
}

// ── Constructor ───────────────────────────────────────────────────────────────
impl OxifsApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        Self::apply_theme(&cc.egui_ctx);

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let (tx, rx) = mpsc::channel();

        let app = Self {
            cwd: cwd.clone(),
            entries: vec![],
            selected: 0,
            scroll_to: false,
            show_hidden: false,
            active_tab: RightTab::Storage,
            clip: Clip::None,
            status: String::new(),
            loading: true,
            disks: disk::get_disks(),
            tx,
            rx,
            search: String::new(),
            search_active: false,
            confirm_delete: false,
            input: None,
        };
        fs_ops::load_dir(cwd, false, app.tx.clone());
        app
    }

    fn apply_theme(ctx: &egui::Context) {
        let mut v = Visuals::dark();
        v.panel_fill          = MANTLE;
        v.window_fill         = BASE;
        v.extreme_bg_color    = CRUST;
        v.code_bg_color       = SURFACE0;
        v.faint_bg_color      = SURFACE0;
        v.hyperlink_color     = BLUE;
        v.window_stroke       = Stroke::new(1.0_f32, SURFACE1);
        v.override_text_color = Some(TEXT);

        v.selection.bg_fill = Color32::from_rgb(72, 44, 64);
        v.selection.stroke  = Stroke::new(1.0_f32, PINK);

        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive] {
            w.bg_fill      = SURFACE0;
            w.weak_bg_fill = SURFACE0;
            w.bg_stroke    = Stroke::new(1.0_f32, SURFACE1);
            w.fg_stroke    = Stroke::new(1.0_f32, TEXT);
        }
        v.widgets.hovered.bg_fill      = SURFACE1;
        v.widgets.hovered.weak_bg_fill = SURFACE1;
        v.widgets.hovered.fg_stroke    = Stroke::new(1.5_f32, PINK);
        v.widgets.active.bg_fill       = SURFACE1;
        v.widgets.active.weak_bg_fill  = SURFACE1;
        v.widgets.active.fg_stroke     = Stroke::new(2.0_f32, PINK);
        v.widgets.open.bg_fill         = SURFACE1;
        v.widgets.open.fg_stroke       = Stroke::new(2.0_f32, PINK);

        ctx.set_visuals(v);
    }
}

// ── Core Navigation & Actions ─────────────────────────────────────────────────
impl OxifsApp {
    fn load_cwd(&mut self) {
        self.loading = true;
        self.status = "Scanning directory…".into();
        fs_ops::load_dir(self.cwd.clone(), self.show_hidden, self.tx.clone());
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                FsMsg::DirLoaded(path, entries) if path == self.cwd => {
                    self.entries = entries;
                    let vis_len = self.visible_entries().len();
                    self.selected = self.selected.min(vis_len.saturating_sub(1));
                    self.loading = false;
                    self.status.clear();
                    self.disks = disk::get_disks();
                }
                FsMsg::DirLoaded(_, _) => {}
                FsMsg::DirSizesComputed(path, sizes) if path == self.cwd => {
                    for (sub_path, size) in sizes {
                        if let Some(entry) = self.entries.iter_mut().find(|e| e.path == sub_path) {
                            entry.size = size;
                        }
                    }
                }
                FsMsg::DirSizesComputed(_, _) => {}
                FsMsg::OpDone(msg) => {
                    self.status = msg;
                    self.load_cwd();
                }
                FsMsg::OpError(msg) => {
                    self.status = format!("Error: {msg}");
                    self.loading = false;
                }
            }
        }
    }

    fn visible_entries(&self) -> Vec<usize> {
        let q = self.search.to_lowercase();
        (0..self.entries.len())
            .filter(|&i| q.is_empty() || self.entries[i].name.to_lowercase().contains(&q))
            .collect()
    }

    fn selected_entry(&self) -> Option<&FileEntry> {
        let vis = self.visible_entries();
        vis.get(self.selected).and_then(|&i| self.entries.get(i))
    }

    fn enter_selected(&mut self) {
        if let Some(e) = self.selected_entry().cloned() {
            if e.is_dir {
                self.cwd = e.path;
                self.selected = 0;
                self.search.clear();
                self.search_active = false;
                self.load_cwd();
            } else {
                let p = e.path.clone();
                std::thread::spawn(move || {
                    let _ = open::that(p);
                });
            }
        }
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.cwd.parent().map(|p| p.to_path_buf()) {
            self.cwd = parent;
            self.selected = 0;
            self.search.clear();
            self.search_active = false;
            self.load_cwd();
        }
    }

    fn do_delete(&mut self) {
        if let Some(e) = self.selected_entry().cloned() {
            self.confirm_delete = false;
            self.loading = true;
            fs_ops::trash_path(e.path, self.tx.clone());
        }
    }

    fn do_paste(&mut self) {
        let cwd = self.cwd.clone();
        let tx = self.tx.clone();
        match self.clip.clone() {
            Clip::Copy(src) => {
                let dest = cwd.join(src.file_name().unwrap_or_default());
                self.loading = true;
                fs_ops::copy_path(src, dest, tx);
            }
            Clip::Cut(src) => {
                let dest = cwd.join(src.file_name().unwrap_or_default());
                self.clip = Clip::None;
                self.loading = true;
                fs_ops::move_path(src, dest, tx);
            }
            Clip::None => {
                self.status = "Clipboard is empty.".into();
            }
        }
    }

    fn commit_input(&mut self) {
        let Some(state) = self.input.take() else { return };
        let tx = self.tx.clone();
        match state.mode {
            InputMode::Rename => {
                if let Some(e) = self.selected_entry().cloned() {
                    let dest = e.path.parent().unwrap_or(&e.path).join(&state.value);
                    self.loading = true;
                    fs_ops::move_path(e.path, dest, tx);
                }
            }
            InputMode::Mkdir => {
                let dir = self.cwd.join(&state.value);
                let name = state.value.clone();
                self.loading = true;
                std::thread::spawn(move || {
                    match std::fs::create_dir_all(&dir) {
                        Ok(_) => {
                            let _ = tx.send(FsMsg::OpDone(format!("Created: {name}")));
                        }
                        Err(e) => {
                            let _ = tx.send(FsMsg::OpError(e.to_string()));
                        }
                    }
                });
            }
        }
    }

    fn yank_path_to_clipboard(&self, ctx: &egui::Context) {
        if let Some(e) = self.selected_entry() {
            let p = e.path.to_string_lossy().into_owned();
            ctx.output_mut(|o| o.copied_text = p.clone());
            let _ = std::process::Command::new("wl-copy").arg(&p).spawn();
        }
    }
}

// ── Keyboard Handling ─────────────────────────────────────────────────────────
impl OxifsApp {
    fn handle_keys(&mut self, ctx: &egui::Context) {
        let modal = self.confirm_delete || self.input.is_some() || self.search_active;
        let vis_count = self.visible_entries().len();

        let esc = ctx.input(|i| i.key_pressed(Key::Escape));
        if esc {
            self.confirm_delete = false;
            self.input = None;
            if self.search_active {
                self.search_active = false;
                self.search.clear();
            }
        }

        if modal {
            return;
        }

        let tab_pressed = ctx.input(|i| i.key_pressed(Key::Tab));
        if tab_pressed {
            self.active_tab = match self.active_tab {
                RightTab::Storage => RightTab::Preview,
                RightTab::Preview => RightTab::Storage,
            };
        }

        ctx.input(|i| {
            if (i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown)) && self.selected + 1 < vis_count {
                self.selected += 1;
                self.scroll_to = true;
            }
            if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp)) && self.selected > 0 {
                self.selected -= 1;
                self.scroll_to = true;
            }
            if i.key_pressed(Key::G) && i.modifiers.shift {
                self.selected = vis_count.saturating_sub(1);
                self.scroll_to = true;
            } else if i.key_pressed(Key::G) {
                self.selected = 0;
                self.scroll_to = true;
            }

            if i.key_pressed(Key::Q) {
                std::process::exit(0);
            }
            if i.key_pressed(Key::Period) {
                self.show_hidden = !self.show_hidden;
                self.load_cwd();
            }
            if i.key_pressed(Key::Slash) {
                self.search_active = true;
            }
            if i.key_pressed(Key::D) {
                self.confirm_delete = true;
            }
            if i.key_pressed(Key::Y) && !i.modifiers.shift {
                if let Some(e) = self.entries.get(self.selected).cloned() {
                    self.status = format!("Copied: {}", e.name);
                    self.clip = Clip::Copy(e.path);
                }
            }
            if i.key_pressed(Key::X) {
                if let Some(e) = self.entries.get(self.selected).cloned() {
                    self.status = format!("Cut: {}", e.name);
                    self.clip = Clip::Cut(e.path);
                }
            }
            if i.key_pressed(Key::R) {
                if let Some(e) = self.selected_entry() {
                    self.input = Some(InputState {
                        mode: InputMode::Rename,
                        value: e.name.clone(),
                    });
                }
            }
            if i.key_pressed(Key::M) {
                self.input = Some(InputState {
                    mode: InputMode::Mkdir,
                    value: String::new(),
                });
            }
        });

        let enter = ctx.input(|i| {
            i.key_pressed(Key::L) || i.key_pressed(Key::Enter) || i.key_pressed(Key::ArrowRight)
        });
        if enter {
            self.enter_selected();
        }

        let up = ctx.input(|i| i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft));
        if up {
            self.go_up();
        }

        let paste = ctx.input(|i| i.key_pressed(Key::P));
        if paste {
            self.do_paste();
        }

        let yank_path = ctx.input(|i| i.key_pressed(Key::Y) && i.modifiers.shift);
        if yank_path {
            self.yank_path_to_clipboard(ctx);
        }
    }
}

// ── Application Layout ────────────────────────────────────────────────────────
impl eframe::App for OxifsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();
        self.handle_keys(ctx);
        if self.loading {
            ctx.request_repaint();
        }

        self.show_top_bar(ctx);
        self.show_status_bar(ctx);
        self.show_file_list(ctx);
        self.show_right_panel(ctx);

        if self.confirm_delete {
            self.show_delete_dialog(ctx);
        }
        if self.input.is_some() {
            self.show_input_dialog(ctx);
        }
        if self.search_active {
            self.show_search_bar(ctx);
        }
    }
}

impl OxifsApp {
    fn show_top_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("topbar")
            .frame(Frame::none().fill(MANTLE).inner_margin(Margin::symmetric(10.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(" 󰁝 Parent ").color(TEXT).strong()).clicked() {
                        self.go_up();
                    }
                    if ui.button(RichText::new(" 󰑐 Refresh ").color(TEXT)).clicked() {
                        self.load_cwd();
                        self.disks = disk::get_disks();
                    }
                    if ui.button(RichText::new(" 󱂵 Home ").color(TEXT)).clicked() {
                        if let Ok(home) = std::env::var("HOME") {
                            self.cwd = PathBuf::from(home);
                            self.selected = 0;
                            self.load_cwd();
                        }
                    }
                    if ui.button(RichText::new(" 󰋊 Root / ").color(TEXT)).clicked() {
                        self.cwd = PathBuf::from("/");
                        self.selected = 0;
                        self.load_cwd();
                    }

                    ui.separator();
                    ui.label(RichText::new("󰉋").color(PINK).size(16.0));
                    ui.label(
                        RichText::new(self.cwd.to_string_lossy().as_ref())
                            .color(PINK)
                            .strong()
                            .size(14.0),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let dot_label = if self.show_hidden {
                            " 󰈉 Hide Dotfiles "
                        } else {
                            " 󰈈 Show Dotfiles "
                        };
                        if ui.button(RichText::new(dot_label).color(SUBTEXT)).clicked() {
                            self.show_hidden = !self.show_hidden;
                            self.load_cwd();
                        }
                        if self.loading {
                            ui.spinner();
                        }
                    });
                });
            });
    }

    fn show_status_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::bottom("statusbar")
            .frame(Frame::none().fill(CRUST).inner_margin(Margin::symmetric(10.0, 5.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let color = if self.status.starts_with("Error") {
                        RED
                    } else {
                        PINK
                    };
                    if self.status.is_empty() {
                        ui.label(
                            RichText::new(
                                "Tab: Switch View | j/k: Navigate | Enter/l: Open | h: Up | d: Trash | y/x/p: Copy/Cut/Paste | /: Search | Y: Yank Path",
                            )
                            .color(OVERLAY)
                            .small(),
                        );
                    } else {
                        ui.label(RichText::new(&self.status).color(color).strong());
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let vis = self.visible_entries().len();
                        let pos = if vis > 0 { self.selected + 1 } else { 0 };
                        ui.label(RichText::new(format!("{pos} / {vis} items")).color(OVERLAY).small());
                    });
                });
            });
    }

    fn show_file_list(&mut self, ctx: &egui::Context) {
        let scroll_to = self.scroll_to;
        self.scroll_to = false;

        SidePanel::left("filelist")
            .min_width(420.0)
            .max_width(580.0)
            .frame(Frame::none().fill(MANTLE).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                // Table header
                Frame::none()
                    .fill(SURFACE0)
                    .inner_margin(Margin::symmetric(12.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Name").color(PINK).small().strong());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new("Size (MB / GB)").color(PINK).small().strong());
                            });
                        });
                    });

                ScrollArea::vertical()
                    .id_salt("file_scroll_view")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let vis = self.visible_entries();
                        let clip_path = match &self.clip {
                            Clip::Copy(p) | Clip::Cut(p) => Some(p.clone()),
                            Clip::None => None,
                        };

                        for (sel_idx, &entry_idx) in vis.iter().enumerate() {
                            let entry = &self.entries[entry_idx];
                            let selected = self.selected == sel_idx;

                            let (name_color, icon) = if entry.is_symlink {
                                (GREEN, "󰌷 ")
                            } else if entry.is_dir {
                                (BLUE, "󰉋 ")
                            } else {
                                (TEXT, file_icon(&entry.name))
                            };

                            let in_clip = clip_path.as_deref() == Some(entry.path.as_path());
                            let clip_tag = if in_clip {
                                match &self.clip {
                                    Clip::Copy(_) => " [Copy]",
                                    Clip::Cut(_) => " [Cut]",
                                    Clip::None => "",
                                }
                            } else {
                                ""
                            };

                            let size_display = if entry.is_dir {
                                if entry.size > 0 {
                                    format!("󰉋 {}", human_size(entry.size))
                                } else {
                                    "󰉋 <dir>".into()
                                }
                            } else {
                                human_size(entry.size)
                            };

                            let bg = if selected {
                                SURFACE1
                            } else {
                                Color32::TRANSPARENT
                            };

                            let resp = Frame::none()
                                .fill(bg)
                                .inner_margin(Margin::symmetric(10.0, 4.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.set_min_width(ui.available_width());
                                        let line_text = format!("{icon}{}{clip_tag}", entry.name);
                                        ui.label(RichText::new(line_text).color(name_color));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(
                                                RichText::new(&size_display)
                                                    .color(if entry.is_dir { OVERLAY } else { YELLOW })
                                                    .small(),
                                            );
                                        });
                                    });
                                })
                                .response;

                            let resp = resp.interact(Sense::click());

                            if resp.clicked() {
                                self.selected = sel_idx;
                            }
                            if resp.double_clicked() {
                                self.selected = sel_idx;
                                self.enter_selected();
                            }
                            if selected && scroll_to {
                                resp.scroll_to_me(Some(Align::Center));
                            }
                        }

                        if self.entries.is_empty() && !self.loading {
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new("Directory is empty").color(OVERLAY));
                            });
                        }
                    });
            });
    }

    fn show_right_panel(&mut self, ctx: &egui::Context) {
        CentralPanel::default()
            .frame(Frame::none().fill(BASE).inner_margin(Margin::same(0.0)))
            .show(ctx, |ui| {
                // View Selector Bar
                Frame::none()
                    .fill(SURFACE0)
                    .inner_margin(Margin::symmetric(12.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let storage_active = self.active_tab == RightTab::Storage;
                            let preview_active = self.active_tab == RightTab::Preview;

                            let storage_btn = RichText::new(" 󰋊 System Storage & Pie Chart ")
                                .color(if storage_active { PINK } else { SUBTEXT })
                                .strong();
                            if ui.selectable_label(storage_active, storage_btn).clicked() {
                                self.active_tab = RightTab::Storage;
                            }

                            let preview_btn = RichText::new(" 󰈙 File Details & Preview ")
                                .color(if preview_active { PINK } else { SUBTEXT })
                                .strong();
                            if ui.selectable_label(preview_active, preview_btn).clicked() {
                                self.active_tab = RightTab::Preview;
                            }
                        });
                    });

                // Panel Content
                ScrollArea::vertical()
                    .id_salt("right_panel_scroll")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        Frame::none()
                            .fill(BASE)
                            .inner_margin(Margin::same(16.0))
                            .show(ui, |ui| match self.active_tab {
                                RightTab::Storage => self.draw_storage_view(ui),
                                RightTab::Preview => self.draw_preview_view(ui),
                            });
                    });
            });
    }

    // ── Tab 1: Storage & Pie Chart View ─────────────────────────────────────────
    fn draw_storage_view(&self, ui: &mut Ui) {
        if self.disks.is_empty() {
            ui.label(RichText::new("No disk storage detected.").color(OVERLAY));
            return;
        }

        let primary_idx = self
            .disks
            .iter()
            .enumerate()
            .filter(|(_, d)| self.cwd.starts_with(&d.mount))
            .max_by_key(|(_, d)| d.mount.len())
            .map(|(i, _)| i)
            .unwrap_or(0);

        let primary = &self.disks[primary_idx];

        ui.label(
            RichText::new("󰋊 Whole System Storage Overview")
                .color(PINK)
                .size(16.0)
                .strong(),
        );
        ui.label(
            RichText::new("System-wide storage capacity, occupied space breakdown, and volume metrics")
                .color(OVERLAY)
                .small(),
        );
        ui.add_space(8.0);

        // Donut Pie Chart + Legend Cards
        ui.horizontal(|ui| {
            let chart_size = 200.0_f32;
            ui.allocate_ui(Vec2::splat(chart_size), |ui| {
                let (resp, painter) = ui.allocate_painter(Vec2::splat(chart_size), Sense::hover());
                let center = resp.rect.center();
                let outer_r = chart_size / 2.0 - 8.0;
                let inner_r = outer_r * 0.58;

                if primary.total_bytes > 0 {
                    let used_ratio = primary.usage_ratio();
                    let start = -FRAC_PI_2;
                    let used_end = start + used_ratio * TAU;

                    draw_donut_sector(&painter, center, inner_r, outer_r, start, used_end, PEACH);
                    draw_donut_sector(&painter, center, inner_r, outer_r, used_end, start + TAU, BLUE);

                    let pct = format!("{:.1}%", used_ratio * 100.0);
                    painter.text(center - Vec2::new(0.0, 8.0), Align2::CENTER_CENTER, &pct, FontId::proportional(22.0), TEXT);
                    painter.text(
                        center + Vec2::new(0.0, 14.0),
                        Align2::CENTER_CENTER,
                        "OCCUPIED",
                        FontId::proportional(11.0),
                        OVERLAY,
                    );
                }
            });

            ui.add_space(16.0);

            // Metrics Cards
            ui.vertical(|ui| {
                ui.label(RichText::new(&primary.name).color(TEXT).size(15.0).strong());
                ui.label(
                    RichText::new(format!("Mounted on: {}  |  Filesystem: {}", primary.mount, primary.fs_type))
                        .color(SUBTEXT)
                        .small(),
                );
                ui.add_space(8.0);

                storage_stat_card(
                    ui,
                    PEACH,
                    "Occupied Storage",
                    &format!("{:.2} GB", primary.used_gb()),
                    &format!("{:.0} MB", primary.used_mb()),
                );
                ui.add_space(4.0);

                storage_stat_card(
                    ui,
                    BLUE,
                    "Available Free Space",
                    &format!("{:.2} GB", primary.avail_gb()),
                    &format!("{:.0} MB", primary.avail_mb()),
                );
                ui.add_space(4.0);

                storage_stat_card(
                    ui,
                    PINK,
                    "Total Capacity",
                    &format!("{:.2} GB", primary.total_gb()),
                    &format!("{:.0} MB", primary.total_mb()),
                );
            });
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        // ── Where Occupied Storage Is Going: Part 1: All Mounted Disks ───────
        ui.label(
            RichText::new("󰉉 Where Occupied Storage Is Going: Mounted Partitions")
                .color(PINK)
                .size(15.0)
                .strong(),
        );
        ui.add_space(6.0);

        for (i, d) in self.disks.iter().enumerate() {
            let color = SLICE_COLORS[i % SLICE_COLORS.len()];
            Frame::none()
                .fill(SURFACE0)
                .inner_margin(Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("󰋊 {}", d.mount)).color(TEXT).strong());
                        ui.label(RichText::new(format!("({})", d.name)).color(OVERLAY).small());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{:.2} GB / {:.2} GB ({:.1}%)",
                                    d.used_gb(),
                                    d.total_gb(),
                                    d.usage_ratio() * 100.0
                                ))
                                .color(color)
                                .strong(),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    let bar_w = ui.available_width();
                    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 8.0), Sense::hover());
                    ui.painter().rect_filled(bar_rect, 4.0, CRUST);
                    let fill_w = bar_rect.width() * d.usage_ratio().clamp(0.0, 1.0);
                    let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_rect.height()));
                    ui.painter().rect_filled(fill_rect, 4.0, color);
                });
            ui.add_space(6.0);
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        // ── Where Occupied Storage Is Going: Part 2: Current Directory Breakdown ─
        ui.label(
            RichText::new("󰉋 Where Occupied Storage Is Going: In Current Directory")
                .color(PINK)
                .size(15.0)
                .strong(),
        );
        ui.label(
            RichText::new(format!(
                "Breakdown of files and subfolders consuming space inside {}",
                self.cwd.to_string_lossy()
            ))
            .color(OVERLAY)
            .small(),
        );
        ui.add_space(8.0);

        self.draw_directory_breakdown(ui);
    }

    fn draw_directory_breakdown(&self, ui: &mut Ui) {
        let mut items: Vec<(String, u64, bool)> = self
            .entries
            .iter()
            .filter(|e| e.size > 0)
            .map(|e| (e.name.clone(), e.size, e.is_dir))
            .collect();

        if items.is_empty() {
            ui.label(RichText::new("No files with measurable disk footprint in this folder.").color(OVERLAY));
            return;
        }

        items.sort_by(|a, b| b.1.cmp(&a.1));

        let total_dir_size: u64 = items.iter().map(|(_, s, _)| *s).sum();
        if total_dir_size == 0 {
            ui.label(RichText::new("Total scanned folder footprint is 0 B.").color(OVERLAY));
            return;
        }

        // Top 6 items + Other
        let top_count = 6.min(items.len());
        let top_items = &items[..top_count];
        let other_size: u64 = items[top_count..].iter().map(|(_, s, _)| *s).sum();

        ui.horizontal(|ui| {
            // Folder pie chart
            let chart_size = 180.0_f32;
            ui.allocate_ui(Vec2::splat(chart_size), |ui| {
                let (resp, painter) = ui.allocate_painter(Vec2::splat(chart_size), Sense::hover());
                let center = resp.rect.center();
                let outer_r = chart_size / 2.0 - 6.0;
                let inner_r = outer_r * 0.52;

                let mut current_ang = -FRAC_PI_2;
                for (i, (_, sz, _)) in top_items.iter().enumerate() {
                    let slice_ratio = *sz as f32 / total_dir_size as f32;
                    let end_ang = current_ang + slice_ratio * TAU;
                    let color = SLICE_COLORS[i % SLICE_COLORS.len()];
                    draw_donut_sector(&painter, center, inner_r, outer_r, current_ang, end_ang, color);
                    current_ang = end_ang;
                }
                if other_size > 0 {
                    let slice_ratio = other_size as f32 / total_dir_size as f32;
                    let end_ang = current_ang + slice_ratio * TAU;
                    draw_donut_sector(&painter, center, inner_r, outer_r, current_ang, end_ang, OVERLAY);
                }

                painter.text(
                    center - Vec2::new(0.0, 6.0),
                    Align2::CENTER_CENTER,
                    &human_size(total_dir_size),
                    FontId::proportional(15.0),
                    TEXT,
                );
                painter.text(
                    center + Vec2::new(0.0, 12.0),
                    Align2::CENTER_CENTER,
                    "FOLDER TOTAL",
                    FontId::proportional(9.0),
                    OVERLAY,
                );
            });

            ui.add_space(14.0);

            // Table of items
            ui.vertical(|ui| {
                for (i, (name, sz, is_dir)) in top_items.iter().enumerate() {
                    let color = SLICE_COLORS[i % SLICE_COLORS.len()];
                    let pct = (*sz as f64 / total_dir_size as f64) * 100.0;
                    let icon = if *is_dir { "󰉋 " } else { "󰈚 " };

                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, color);
                        ui.label(RichText::new(format!("{icon}{name}")).color(TEXT).small());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{:.1}% ({})", pct, human_size(*sz)))
                                    .color(color)
                                    .small()
                                    .strong(),
                            );
                        });
                    });
                    ui.add_space(2.0);
                }

                if other_size > 0 {
                    let pct = (other_size as f64 / total_dir_size as f64) * 100.0;
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, OVERLAY);
                        ui.label(RichText::new("Other files").color(SUBTEXT).small());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{:.1}% ({})", pct, human_size(other_size)))
                                    .color(OVERLAY)
                                    .small(),
                            );
                        });
                    });
                }
            });
        });
    }

    // ── Tab 2: File Details & Preview View ───────────────────────────────────────
    fn draw_preview_view(&self, ui: &mut Ui) {
        let entry = match self.selected_entry() {
            Some(e) => e,
            None => {
                ui.label(RichText::new("Select a file or folder to view details.").color(OVERLAY));
                return;
            }
        };

        ui.label(RichText::new("󰈙 File & Memory Specifications").color(PINK).size(16.0).strong());
        ui.add_space(6.0);

        Frame::none()
            .fill(SURFACE0)
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                Grid::new("file_meta_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        let mut row = |label: &str, val: &str, col: Color32| {
                            ui.label(RichText::new(label).color(OVERLAY).small());
                            ui.label(RichText::new(val).color(col).strong());
                            ui.end_row();
                        };

                        row("Name", &entry.name, TEXT);
                        row("Full Path", &entry.path.to_string_lossy(), SUBTEXT);
                        row(
                            "Type",
                            if entry.is_symlink {
                                "Symbolic Link"
                            } else if entry.is_dir {
                                "Directory"
                            } else {
                                "Regular File"
                            },
                            TEXT,
                        );

                        // File memory / storage specifications in MB and GB
                        row("Memory in MB", &format_mb(entry.size), YELLOW);
                        row("Memory in GB", &format_gb(entry.size), PEACH);
                        row("Exact Raw Bytes", &format!("{} bytes", entry.size), BLUE);

                        if let Some(m) = &entry.modified {
                            row("Last Modified", &m.format("%Y-%m-%d  %H:%M:%S").to_string(), SUBTEXT);
                        }
                    });
            });

        ui.add_space(14.0);

        // Content Preview for readable files
        if !entry.is_dir && entry.size > 0 && entry.size < 65_536 {
            if let Ok(text) = std::fs::read_to_string(&entry.path) {
                ui.label(RichText::new("󰈙 File Text Preview").color(PINK).strong());
                ui.add_space(4.0);
                Frame::none()
                    .fill(CRUST)
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("text_preview_scroll")
                            .max_height(340.0)
                            .show(ui, |ui| {
                                for (idx, line) in text.lines().enumerate().take(200) {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("{:>4} ", idx + 1))
                                                .color(OVERLAY)
                                                .monospace()
                                                .small(),
                                        );
                                        ui.label(
                                            RichText::new(line)
                                                .color(TEXT)
                                                .monospace()
                                                .small(),
                                        );
                                    });
                                }
                            });
                    });
            }
        }
    }

    // ── Dialog Modals ───────────────────────────────────────────────────────────
    fn show_delete_dialog(&mut self, ctx: &egui::Context) {
        let name = self
            .selected_entry()
            .map(|e| e.name.clone())
            .unwrap_or_default();

        Window::new("Confirm Move to Trash")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(Frame::window(&ctx.style()).fill(BASE).stroke(Stroke::new(1.0_f32, RED)))
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("Are you sure you want to move this item to Trash?").color(TEXT));
                ui.label(RichText::new(&name).color(RED).strong());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(" Yes, Trash it ").color(RED)).clicked() {
                        self.do_delete();
                    }
                    if ui.button(RichText::new(" Cancel ").color(SUBTEXT)).clicked() {
                        self.confirm_delete = false;
                    }
                });
                ui.add_space(4.0);
            });
    }

    fn show_input_dialog(&mut self, ctx: &egui::Context) {
        let (title, prompt) = match self.input.as_ref().map(|s| &s.mode) {
            Some(InputMode::Rename) => ("Rename Item", "Enter new name:"),
            Some(InputMode::Mkdir) => ("Create Directory", "Enter folder name:"),
            None => return,
        };

        let mut commit = false;
        let mut cancel = false;

        Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(Frame::window(&ctx.style()).fill(BASE).stroke(Stroke::new(1.0_f32, PINK)))
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(RichText::new(prompt).color(SUBTEXT));
                if let Some(state) = &mut self.input {
                    let resp = ui.add(
                        TextEdit::singleline(&mut state.value)
                            .desired_width(280.0)
                            .text_color(TEXT),
                    );
                    resp.request_focus();
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        commit = true;
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(" OK ").color(PINK)).clicked() {
                        commit = true;
                    }
                    if ui.button(RichText::new(" Cancel ").color(SUBTEXT)).clicked() {
                        cancel = true;
                    }
                });
                ui.add_space(4.0);
            });

        if commit {
            self.commit_input();
        }
        if cancel {
            self.input = None;
        }
    }

    fn show_search_bar(&mut self, ctx: &egui::Context) {
        Window::new("Quick Search")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, [10.0, -38.0])
            .frame(Frame::none().fill(SURFACE1).inner_margin(Margin::symmetric(10.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("󰍉 /").color(GREEN).strong());
                    let resp = ui.add(
                        TextEdit::singleline(&mut self.search)
                            .desired_width(240.0)
                            .text_color(TEXT)
                            .frame(false),
                    );
                    resp.request_focus();
                    self.selected = 0;
                    if resp.lost_focus() {
                        self.search_active = false;
                    }
                    let count = self.visible_entries().len();
                    ui.label(RichText::new(format!("({count} matches)")).color(OVERLAY).small());
                });
            });
    }
}

// ── Donut Sector Vector Drawing ───────────────────────────────────────────────
fn draw_donut_sector(
    painter: &Painter,
    center: Pos2,
    inner_r: f32,
    outer_r: f32,
    start: f32,
    end: f32,
    color: Color32,
) {
    let span = end - start;
    if span.abs() < 0.005 {
        return;
    }
    let gap = 0.02_f32;
    let a0 = start + gap;
    let a1 = end - gap;
    if a1 <= a0 {
        return;
    }
    let steps = ((span.abs() * 32.0 / TAU) as usize + 2).min(48);
    let mut points = Vec::with_capacity(steps * 2 + 2);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = a0 + t * (a1 - a0);
        points.push(Pos2::new(
            center.x + outer_r * angle.cos(),
            center.y + outer_r * angle.sin(),
        ));
    }
    for i in (0..=steps).rev() {
        let t = i as f32 / steps as f32;
        let angle = a0 + t * (a1 - a0);
        points.push(Pos2::new(
            center.x + inner_r * angle.cos(),
            center.y + inner_r * angle.sin(),
        ));
    }

    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn storage_stat_card(ui: &mut Ui, color: Color32, label: &str, gb: &str, mb: &str) {
    Frame::none()
        .fill(SURFACE0)
        .inner_margin(Margin::symmetric(10.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                ui.painter().rect_filled(rect, 3.0, color);
                ui.label(RichText::new(label).color(TEXT).small());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("{gb} ({mb})")).color(color).strong());
                });
            });
        });
}

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "󱘗 ",
        "toml" | "yaml" | "yml" => "󰬔 ",
        "md" => "󰍔 ",
        "sh" | "bash" | "zsh" | "fish" => "󰆍 ",
        "py" => "󰌠 ",
        "js" | "ts" => "󰌞 ",
        "json" => "󰘦 ",
        "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" => "󰋩 ",
        "mp4" | "mkv" | "webm" | "mov" => "󰎁 ",
        "mp3" | "flac" | "ogg" | "wav" => "󰝚 ",
        "zip" | "tar" | "gz" | "xz" | "bz2" | "zst" => "󰛫 ",
        "pdf" => "󰈦 ",
        "txt" => "󰈙 ",
        "html" | "htm" => "󰌾 ",
        "css" => "󰌜 ",
        _ => "󰈚 ",
    }
}
