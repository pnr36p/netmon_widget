use crate::config::save_config;
use crate::i18n::{tr, Language, ThemeMode};
use crate::state::AppState;
use eframe::egui::{self, Color32, ViewportCommand, ViewportId};

pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    if !state.show_settings {
        return;
    }

    let s = tr(state.config.language);
    let mut builder = egui::ViewportBuilder::default()
        .with_title(s.settings_title)
        .with_inner_size([390.0, 615.0])
        .with_min_inner_size([350.0, 540.0])
        .with_resizable(false)
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
            ctx.set_visuals(if is_dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });

            if applied_initial.is_none() {
                if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                    if outer.width() > 50.0 && outer.min.x.is_finite() && outer.min.y.is_finite() {
                        state.config.settings_window_pos = Some([outer.min.x, outer.min.y]);
                    }
                }
            }

            if ctx.input(|i| i.viewport().close_requested()) {
                state.config.opacity = state.opacity;
                save_config(&state.config);
                state.show_settings = false;
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                let s = tr(state.config.language);
                ui.spacing_mut().item_spacing = egui::Vec2::new(8.0, 7.0);

                ui.heading(s.settings_heading);
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(s.settings_language);
                    let mut lang = state.config.language;
                    egui::ComboBox::from_id_salt("settings_lang_combo")
                        .width(140.0)
                        .selected_text(lang.display_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut lang, Language::Ru, Language::Ru.display_name());
                            ui.selectable_value(&mut lang, Language::En, Language::En.display_name());
                            ui.selectable_value(&mut lang, Language::Hr, Language::Hr.display_name());
                        });
                    if lang != state.config.language {
                        state.config.language = lang;
                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::Title(tr(lang).settings_title.to_string()));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(s.settings_theme);
                    let theme_name = |t: ThemeMode| match t {
                        ThemeMode::System => s.theme_system,
                        ThemeMode::Dark => s.theme_dark,
                        ThemeMode::Light => s.theme_light,
                    };
                    let mut theme = state.config.theme;
                    egui::ComboBox::from_id_salt("settings_theme_combo")
                        .width(140.0)
                        .selected_text(theme_name(theme))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut theme, ThemeMode::System, s.theme_system);
                            ui.selectable_value(&mut theme, ThemeMode::Dark, s.theme_dark);
                            ui.selectable_value(&mut theme, ThemeMode::Light, s.theme_light);
                        });
                    state.config.theme = theme;
                });

                ui.add_space(2.0);
                ui.label(s.settings_size);
                let mut size = state.config.size;
                let old_size = size;
                egui::ComboBox::from_id_salt("settings_size_combo")
                    .selected_text(match size {
                        crate::config::SizePreset::Xs => "XS (120 px)",
                        crate::config::SizePreset::S => "S (130 px - Windows 7 Gadget)",
                        crate::config::SizePreset::M => "M (150 px)",
                        crate::config::SizePreset::L => "L (170 px)",
                        crate::config::SizePreset::Xl => "XL (200 px)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut size, crate::config::SizePreset::Xs, "XS (120 px)");
                        ui.selectable_value(
                            &mut size,
                            crate::config::SizePreset::S,
                            "S (130 px - Windows 7 Gadget)",
                        );
                        ui.selectable_value(&mut size, crate::config::SizePreset::M, "M (150 px)");
                        ui.selectable_value(&mut size, crate::config::SizePreset::L, "L (170 px)");
                        ui.selectable_value(&mut size, crate::config::SizePreset::Xl, "XL (200 px)");
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
                            .map(|sec| size.header_height() + sec.checks.len() as f32 * size.row_height())
                            .sum::<f32>()
                        + 4.0;
                    ui.ctx().send_viewport_cmd_to(
                        ViewportId::ROOT,
                        ViewportCommand::InnerSize(egui::Vec2::new(
                            new_width,
                            current_height.clamp(120.0, 1200.0),
                        )),
                    );
                }

                ui.add_space(2.0);
                ui.label(s.settings_opacity);
                ui.add(egui::Slider::new(&mut state.opacity, 0.2..=1.0).show_value(true));

                ui.add_space(2.0);
                ui.label(s.settings_interval);
                let mut interval = state.config.interval_sec;
                if ui
                    .add(egui::Slider::new(&mut interval, 1..=60).text(s.settings_sec_unit))
                    .changed()
                {
                    state.config.interval_sec = interval;
                    state.scheduler_needs_sync = true;
                }

                ui.add_space(2.0);
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
                if ui.checkbox(&mut edge_snap, s.settings_edge_snapping).changed() {
                    state.config.edge_snapping = edge_snap;
                    crate::platform::set_edge_snapping(edge_snap);
                    save_config(&state.config);
                }

                let mut hide_taskbar = state.config.hide_taskbar;
                if ui.checkbox(&mut hide_taskbar, s.settings_hide_taskbar).changed() {
                    state.config.hide_taskbar = hide_taskbar;
                    crate::platform::set_skip_taskbar(hide_taskbar);
                    save_config(&state.config);
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

                let mut auto_upd = state.config.auto_update;
                if ui.checkbox(&mut auto_upd, s.settings_auto_update).changed() {
                    state.config.auto_update = auto_upd;
                    state.scheduler_needs_sync = true;
                    save_config(&state.config);
                }

                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} v{}",
                        s.update_version_prefix,
                        crate::updater::CURRENT_VERSION
                    ));
                    match &state.update_status {
                        crate::updater::UpdateStatus::Idle => {
                            if ui.button(s.update_check_now).clicked() {
                                state.manual_update_check_requested = true;
                            }
                        }
                        crate::updater::UpdateStatus::Checking => {
                            ui.spinner();
                            ui.label(s.update_checking);
                        }
                        crate::updater::UpdateStatus::Downloading(ver) => {
                            ui.spinner();
                            ui.label(format!("{} (v{})", s.update_downloading, ver));
                        }
                        crate::updater::UpdateStatus::UpToDate => {
                            ui.colored_label(
                                Color32::from_rgb(0x3D, 0xA8, 0x52),
                                s.update_up_to_date,
                            );
                            if ui.button(s.update_check_now).clicked() {
                                state.manual_update_check_requested = true;
                            }
                        }
                        crate::updater::UpdateStatus::ReadyToRestart(ver) => {
                            ui.colored_label(
                                Color32::from_rgb(0x3D, 0xA8, 0x52),
                                s.update_ready.replace("{}", ver),
                            );
                            if ui.button(s.update_restart_btn).clicked() {
                                state.restart_for_update_requested = true;
                            }
                        }
                        crate::updater::UpdateStatus::Error(err) => {
                            if ui.button(s.update_check_now).clicked() {
                                state.manual_update_check_requested = true;
                            }
                            let short = if err.chars().count() > 36 {
                                let t: String = err.chars().take(35).collect();
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
                if ui.button(s.settings_open_toml).clicked() {
                    let path = crate::config::config_path();
                    let _ = std::process::Command::new("notepad.exe").arg(path).spawn();
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(s.btn_save).clicked() {
                        state.config.opacity = state.opacity;
                        save_config(&state.config);
                        state.show_settings = false;
                    }
                    if ui.button(s.btn_close).clicked() {
                        state.config.opacity = state.opacity;
                        save_config(&state.config);
                        state.show_settings = false;
                    }
                });
            });
        },
    );
}
