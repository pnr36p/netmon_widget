use crate::config::SizePreset;
use crate::i18n::{tr, Language, ThemeMode};
use crate::state::AppState;
use crate::updater::{UpdateStatus, CURRENT_VERSION};
use eframe::egui::{
    self, Color32, CornerRadius, Margin, RichText, Stroke, Vec2, ViewportCommand, ViewportId,
};

const COMBO_WIDTH: f32 = 205.0;
const SLIDER_WIDTH: f32 = 148.0;

pub fn window_size_for_language(lang: Language) -> Vec2 {
    match lang {
        Language::En => Vec2::new(425.0, 600.0),
        Language::Ru => Vec2::new(470.0, 605.0),
        Language::Hr => Vec2::new(460.0, 605.0),
    }
}

fn max_dialog_height_for(main_rect: Option<egui::Rect>) -> f32 {
    let monitors = crate::monitor::get_all_monitors();
    let monitor = main_rect
        .and_then(|r| {
            monitors.iter().max_by(|a, b| {
                let la = a.logical_work_rect().intersect(r);
                let lb = b.logical_work_rect().intersect(r);
                let area_a = la.width().max(0.0) * la.height().max(0.0);
                let area_b = lb.width().max(0.0) * lb.height().max(0.0);
                area_a
                    .partial_cmp(&area_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
        .or_else(|| monitors.iter().find(|m| m.is_primary))
        .or_else(|| monitors.first());

    monitor
        .map(|m| (m.logical_work_rect().height() - 44.0).max(400.0))
        .unwrap_or(900.0)
}

pub fn clamped_window_size(lang: Language, main_rect: Option<egui::Rect>) -> Vec2 {
    let mut size = window_size_for_language(lang);
    size.y = size.y.min(max_dialog_height_for(main_rect));
    size
}

fn size_preset_label(size: SizePreset) -> &'static str {
    match size {
        SizePreset::Xs => "XS (120 px)",
        SizePreset::S => "S (130 px - Windows 7 Gadget)",
        SizePreset::M => "M (150 px)",
        SizePreset::L => "L (170 px)",
        SizePreset::Xl => "XL (200 px)",
    }
}

fn section_card(
    ui: &mut egui::Ui,
    is_dark: bool,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let (fill, stroke, title_color) = if is_dark {
        (
            Color32::from_rgb(0x27, 0x29, 0x2F),
            Color32::from_rgb(0x39, 0x3C, 0x45),
            Color32::from_rgb(0x9E, 0xB4, 0xD8),
        )
    } else {
        (
            Color32::from_rgb(0xF6, 0xF7, 0xFA),
            Color32::from_rgb(0xDC, 0xE1, 0xEA),
            Color32::from_rgb(0x36, 0x5B, 0x99),
        )
    };

    egui::Frame::NONE
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(title).size(12.5).strong().color(title_color));
            ui.add_space(3.0);
            add_contents(ui);
        });
}

fn accent_button(ui: &mut egui::Ui, text: &str, fill: Color32) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(Color32::WHITE).strong())
            .fill(fill)
            .corner_radius(CornerRadius::same(6)),
    )
}

pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_settings {
        return;
    }

    let s = tr(state.config.language);
    let default_size = clamped_window_size(state.config.language, state.main_outer_rect);
    let mut builder = egui::ViewportBuilder::default()
        .with_title(s.settings_title)
        .with_inner_size([default_size.x, default_size.y])
        .with_min_inner_size([390.0, 400.0])
        .with_resizable(true)
        .with_always_on_top();

    let applied_initial = state.settings_open_pos.take();
    if let Some(pos) = applied_initial {
        builder = builder.with_position(pos);
    }

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("settings_window_dialog"),
        builder,
        |ctx, _class| {
            let is_dark = state.config.theme.resolve_is_dark();
            let mut visuals = if is_dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
            visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
            visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
            visuals.widgets.active.corner_radius = CornerRadius::same(6);
            ctx.set_visuals(visuals);

            if applied_initial.is_none() {
                if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                    if outer.width() > 50.0 && outer.min.x.is_finite() && outer.min.y.is_finite() {
                        state.config.settings_window_pos = Some([outer.min.x, outer.min.y]);
                    }
                }
            }

            if ctx.input(|i| i.viewport().close_requested()) {
                state.save_settings_to_disk();
                state.show_settings = false;
            }

            let panel_bg = if is_dark {
                Color32::from_rgb(0x1E, 0x20, 0x25)
            } else {
                Color32::from_rgb(0xEC, 0xEE, 0xF3)
            };
            let footer_bg = if is_dark {
                Color32::from_rgb(0x23, 0x25, 0x2B)
            } else {
                Color32::from_rgb(0xE4, 0xE7, 0xEE)
            };
            let footer_border = if is_dark {
                Color32::from_rgb(0x36, 0x39, 0x42)
            } else {
                Color32::from_rgb(0xD2, 0xD7, 0xE1)
            };

            let s = tr(state.config.language);

            // Bottom action bar docked at window bottom.
            egui::Panel::bottom("settings_footer")
                .frame(
                    egui::Frame::NONE
                        .fill(footer_bg)
                        .stroke(Stroke::new(1.0, footer_border))
                        .inner_margin(Margin::symmetric(14, 10)),
                )
                .show(ctx, |ui| {
                    ui.spacing_mut().button_padding = Vec2::new(14.0, 6.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(s.btn_close).clicked() {
                            state.save_settings_to_disk();
                            state.show_settings = false;
                        }
                        if accent_button(
                            ui,
                            s.btn_save,
                            Color32::from_rgb(0x2B, 0x6C, 0xDF),
                        )
                        .clicked()
                        {
                            state.save_settings_to_disk();
                            state.show_settings = false;
                        }
                    });
                });

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(panel_bg)
                        .inner_margin(Margin::symmetric(14, 10)),
                )
                .show(ctx, |ui| {
                    let s = tr(state.config.language);
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 7.0);
                    ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
                    ui.spacing_mut().slider_width = SLIDER_WIDTH;

                    ui.label(RichText::new(s.settings_heading).size(16.5).strong());

                    let main_rect = state.main_outer_rect;
                    let scroll_out = egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
                            // 1. Appearance
                            section_card(ui, is_dark, s.settings_sec_appearance, |ui| {
                                egui::Grid::new("settings_appearance_grid")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(s.settings_language);
                                        let mut lang = state.config.language;
                                        egui::ComboBox::from_id_salt("settings_lang_combo")
                                            .width(COMBO_WIDTH)
                                            .selected_text(lang.display_name())
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut lang,
                                                    Language::Ru,
                                                    Language::Ru.display_name(),
                                                );
                                                ui.selectable_value(
                                                    &mut lang,
                                                    Language::En,
                                                    Language::En.display_name(),
                                                );
                                                ui.selectable_value(
                                                    &mut lang,
                                                    Language::Hr,
                                                    Language::Hr.display_name(),
                                                );
                                            });
                                        if lang != state.config.language {
                                            state.config.language = lang;
                                            ui.ctx().send_viewport_cmd(ViewportCommand::Title(
                                                tr(lang).settings_title.to_string(),
                                            ));
                                            ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(
                                                clamped_window_size(lang, main_rect),
                                            ));
                                        }
                                        ui.end_row();

                                        ui.label(s.settings_theme);
                                        let theme_name = |t: ThemeMode| match t {
                                            ThemeMode::System => s.theme_system,
                                            ThemeMode::Dark => s.theme_dark,
                                            ThemeMode::Light => s.theme_light,
                                        };
                                        let mut theme = state.config.theme;
                                        egui::ComboBox::from_id_salt("settings_theme_combo")
                                            .width(COMBO_WIDTH)
                                            .selected_text(theme_name(theme))
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut theme,
                                                    ThemeMode::System,
                                                    s.theme_system,
                                                );
                                                ui.selectable_value(
                                                    &mut theme,
                                                    ThemeMode::Dark,
                                                    s.theme_dark,
                                                );
                                                ui.selectable_value(
                                                    &mut theme,
                                                    ThemeMode::Light,
                                                    s.theme_light,
                                                );
                                            });
                                        state.config.theme = theme;
                                        ui.end_row();

                                        ui.label(s.settings_size);
                                        let mut size = state.config.size;
                                        let old_size = size;
                                        egui::ComboBox::from_id_salt("settings_size_combo")
                                            .width(COMBO_WIDTH)
                                            .selected_text(size_preset_label(size))
                                            .show_ui(ui, |ui| {
                                                for preset in [
                                                    SizePreset::Xs,
                                                    SizePreset::S,
                                                    SizePreset::M,
                                                    SizePreset::L,
                                                    SizePreset::Xl,
                                                ] {
                                                    ui.selectable_value(
                                                        &mut size,
                                                        preset,
                                                        size_preset_label(preset),
                                                    );
                                                }
                                            });
                                        if size != old_size {
                                            state.config.size = size;
                                            state.config.custom_width = None;
                                            state.config.custom_height = None;
                                            let new_width = size.width();
                                            let current_height = 22.0
                                                + state
                                                    .sections
                                                    .iter()
                                                    .map(|sec| {
                                                        size.header_height()
                                                            + sec.checks.len() as f32
                                                                * size.row_height()
                                                    })
                                                    .sum::<f32>()
                                                + 4.0;
                                            ui.ctx().send_viewport_cmd_to(
                                                ViewportId::ROOT,
                                                ViewportCommand::InnerSize(Vec2::new(
                                                    new_width,
                                                    current_height.clamp(120.0, 1200.0),
                                                )),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label(s.settings_opacity);
                                        if ui
                                            .add(
                                                egui::Slider::new(&mut state.opacity, 0.2..=1.0)
                                                    .fixed_decimals(2)
                                                    .show_value(true),
                                            )
                                            .changed()
                                        {
                                            state.config.opacity = state.opacity;
                                        }
                                        ui.end_row();
                                    });
                            });

                            // 2. Behavior & Polling
                            section_card(ui, is_dark, s.settings_sec_behavior, |ui| {
                                egui::Grid::new("settings_polling_grid")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label(s.settings_interval);
                                        let mut interval = state.config.interval_sec;
                                        if ui
                                            .add(
                                                egui::Slider::new(&mut interval, 1..=60)
                                                    .text(s.settings_sec_unit),
                                            )
                                            .changed()
                                        {
                                            state.config.interval_sec = interval;
                                            state.scheduler_needs_sync = true;
                                        }
                                        ui.end_row();
                                    });

                                ui.add_space(4.0);

                                let mut aot = state.config.always_on_top;
                                if ui.checkbox(&mut aot, s.settings_always_on_top).changed() {
                                    state.config.always_on_top = aot;
                                    ui.ctx().send_viewport_cmd_to(
                                        ViewportId::ROOT,
                                        ViewportCommand::WindowLevel(if aot {
                                            egui::WindowLevel::AlwaysOnTop
                                        } else {
                                            egui::WindowLevel::Normal
                                        }),
                                    );
                                }

                                let mut edge_snap = state.config.edge_snapping;
                                if ui
                                    .checkbox(&mut edge_snap, s.settings_edge_snapping)
                                    .changed()
                                {
                                    state.config.edge_snapping = edge_snap;
                                    crate::platform::set_edge_snapping(edge_snap);
                                    state.save_settings_to_disk();
                                }

                                let mut hide_taskbar = state.config.hide_taskbar;
                                if ui
                                    .checkbox(&mut hide_taskbar, s.settings_hide_taskbar)
                                    .changed()
                                {
                                    state.config.hide_taskbar = hide_taskbar;
                                    crate::platform::set_skip_taskbar(hide_taskbar);
                                    state.save_settings_to_disk();
                                }

                                let mut show_lat = state.config.show_latency_in_list;
                                if ui.checkbox(&mut show_lat, s.settings_show_latency).changed() {
                                    state.config.show_latency_in_list = show_lat;
                                    state.save_settings_to_disk();
                                }

                                let mut auto = state.autostart_enabled;
                                if ui.checkbox(&mut auto, s.settings_autostart).changed() {
                                    match crate::platform::set_autostart(auto) {
                                        Ok(()) => {
                                            state.autostart_enabled = auto;
                                            state.autostart_error = None;
                                        }
                                        Err(e) => {
                                            state.autostart_error = Some(e);
                                        }
                                    }
                                }
                                if let Some(err) = &state.autostart_error {
                                    ui.colored_label(Color32::from_rgb(0xE0, 0x45, 0x45), err);
                                }
                            });

                            // 3. Updates
                            section_card(ui, is_dark, s.settings_sec_updates, |ui| {
                                let mut auto_upd = state.config.auto_update;
                                if ui.checkbox(&mut auto_upd, s.settings_auto_update).changed() {
                                    state.config.auto_update = auto_upd;
                                    state.scheduler_needs_sync = true;
                                    state.save_settings_to_disk();
                                }

                                ui.add_space(2.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} v{}",
                                            s.update_version_prefix, CURRENT_VERSION
                                        ))
                                        .strong(),
                                    );
                                    match &state.update_status {
                                        UpdateStatus::Idle => {}
                                        UpdateStatus::Checking => {
                                            ui.spinner();
                                            ui.label(s.update_checking);
                                        }
                                        UpdateStatus::Downloading(ver) => {
                                            ui.spinner();
                                            ui.label(format!("{} (v{})", s.update_downloading, ver));
                                        }
                                        UpdateStatus::UpToDate => {
                                            ui.colored_label(
                                                Color32::from_rgb(0x3D, 0xA8, 0x52),
                                                s.update_up_to_date,
                                            );
                                        }
                                        UpdateStatus::ReadyToRestart(ver) => {
                                            ui.colored_label(
                                                Color32::from_rgb(0x3D, 0xA8, 0x52),
                                                s.update_ready.replace("{}", ver),
                                            );
                                        }
                                        UpdateStatus::Error(err) => {
                                            let short = if err.chars().count() > 60 {
                                                let t: String = err.chars().take(59).collect();
                                                format!("{}…", t)
                                            } else {
                                                err.clone()
                                            };
                                            ui.colored_label(
                                                Color32::from_rgb(0xE0, 0x45, 0x45),
                                                format!("{}{}", s.update_err_prefix, short),
                                            );
                                        }
                                    }
                                });

                                ui.add_space(4.0);
                                ui.horizontal(|ui| match &state.update_status {
                                    UpdateStatus::Idle
                                    | UpdateStatus::UpToDate
                                    | UpdateStatus::Error(_) => {
                                        if ui.button(s.update_check_now).clicked() {
                                            state.manual_update_check_requested = true;
                                        }
                                    }
                                    UpdateStatus::Checking | UpdateStatus::Downloading(_) => {
                                        ui.add_enabled(false, egui::Button::new(s.update_check_now));
                                    }
                                    UpdateStatus::ReadyToRestart(_) => {
                                        if accent_button(
                                            ui,
                                            s.update_restart_btn,
                                            Color32::from_rgb(0x2E, 0x8B, 0x47),
                                        )
                                        .clicked()
                                        {
                                            state.restart_for_update_requested = true;
                                        }
                                    }
                                });
                            });

                            if ui.button(s.settings_open_toml).clicked() {
                                let path = crate::config::config_path();
                                let _ = std::process::Command::new("notepad.exe").arg(path).spawn();
                            }
                        });

                    // Auto-expand window height if content overflows (up to monitor work area),
                    // so everything fits without a scrollbar whenever screen height permits.
                    let overflow = scroll_out.content_size.y - scroll_out.inner_rect.height();
                    if overflow > 1.5 {
                        if let Some(cur) = ui.ctx().input(|i| i.viewport().inner_rect) {
                            let max_h = max_dialog_height_for(main_rect);
                            let target_h = (cur.height() + overflow + 4.0).min(max_h);
                            if target_h > cur.height() + 2.0 {
                                ui.ctx().send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
                                    cur.width(),
                                    target_h,
                                )));
                            }
                        }
                    }
                });
        },
    );
}
