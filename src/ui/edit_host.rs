use crate::config::{CheckConfig, SectionConfig};
use crate::i18n::{tr, Language};
use crate::state::{AppState, CheckState, EditHostState, SectionState};
use eframe::egui::{self, Color32, ViewportCommand, ViewportId};

fn kind_label(kind: &str, lang: Language) -> &'static str {
    match (kind, lang) {
        ("ping", _) => "ping (ICMP)",
        ("tcp", Language::Ru) => "tcp (TCP-порт)",
        ("tcp", Language::En) => "tcp (TCP port)",
        ("tcp", Language::Hr) => "tcp (TCP port)",
        ("https", _) => "https (HTTPS GET)",
        ("http", _) => "http (HTTP GET)",
        ("dns", Language::Ru) => "dns (DNS-запрос)",
        ("dns", Language::En) => "dns (DNS query)",
        ("dns", Language::Hr) => "dns (DNS upit)",
        _ => "ping (ICMP)",
    }
}

fn track_edit_window_pos(ctx: &egui::Context, state: &mut AppState, first_frame: bool) {
    if !first_frame {
        if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
            if outer.width() > 50.0 && outer.min.x.is_finite() && outer.min.y.is_finite() {
                state.config.edit_window_pos = Some([outer.min.x, outer.min.y]);
            }
        }
    }
}

pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    let Some(mut edit) = state.editing_check.take() else {
        return;
    };

    let lang = state.config.language;
    let s = tr(lang);
    let title = if edit.is_new {
        s.new_host_title
    } else {
        s.edit_host_title
    };

    let mut builder = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size([360.0, 435.0])
        .with_min_inner_size([330.0, 395.0])
        .with_resizable(false)
        .with_always_on_top();

    let applied_initial = state.edit_open_pos.take();
    if let Some(pos) = applied_initial {
        builder = builder.with_position(pos);
    }

    let mut keep_open = true;
    let section_names: Vec<String> = state.config.sections.iter().map(|sec| sec.name.clone()).collect();
    let is_dark = state.config.theme.resolve_is_dark();

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("edit_host_dialog"),
        builder,
        |ctx, _class| {
            ctx.set_visuals(if is_dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            track_edit_window_pos(ctx, state, applied_initial.is_some());

            if ctx.input(|i| i.viewport().close_requested()) {
                keep_open = false;
            }
            let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            if esc_pressed {
                keep_open = false;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::new(8.0, 7.0);
                ui.heading(s.host_params_heading);
                ui.add_space(2.0);

                egui::Grid::new("edit_host_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(s.field_section);
                        if !section_names.is_empty() {
                            if edit.target_section_idx >= section_names.len() {
                                edit.target_section_idx = section_names.len() - 1;
                            }
                            egui::ComboBox::from_id_salt("edit_host_section")
                                .width(210.0)
                                .selected_text(&section_names[edit.target_section_idx])
                                .show_ui(ui, |ui| {
                                    for (idx, sec_name) in section_names.iter().enumerate() {
                                        ui.selectable_value(&mut edit.target_section_idx, idx, sec_name);
                                    }
                                });
                        } else {
                            ui.label("—");
                        }
                        ui.end_row();

                        ui.label(s.field_name);
                        ui.add(egui::TextEdit::singleline(&mut edit.name).desired_width(210.0));
                        ui.end_row();

                        ui.label(s.field_host);
                        ui.add(egui::TextEdit::singleline(&mut edit.host).desired_width(210.0));
                        ui.end_row();

                        ui.label(s.field_kind);
                        egui::ComboBox::from_id_salt("edit_host_kind")
                            .width(210.0)
                            .selected_text(kind_label(&edit.base_kind, lang))
                            .show_ui(ui, |ui| {
                                for k in ["ping", "tcp", "https", "http", "dns"] {
                                    ui.selectable_value(
                                        &mut edit.base_kind,
                                        k.to_string(),
                                        kind_label(k, lang),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label(s.field_port);
                        ui.vertical(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut edit.port_str)
                                    .desired_width(120.0)
                                    .hint_text(s.hint_port_auto),
                            );
                            ui.small(edit.default_port_hint(lang));
                        });
                        ui.end_row();

                        ui.label(s.field_timeout);
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.timeout_str)
                                .desired_width(120.0)
                                .hint_text("2000"),
                        );
                        ui.end_row();

                        ui.label(s.field_host_interval);
                        ui.add(
                            egui::TextEdit::singleline(&mut edit.interval_str)
                                .desired_width(120.0)
                                .hint_text(s.hint_interval_global),
                        );
                        ui.end_row();
                    });

                ui.add_space(4.0);
                ui.checkbox(&mut edit.optional, s.check_optional);
                ui.checkbox(&mut edit.notify, s.check_notify);

                if let Some(err) = &edit.error_msg {
                    ui.colored_label(Color32::from_rgb(0xFF, 0x6B, 0x6B), err);
                }

                ui.add_space(6.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button(s.btn_save).clicked() || (enter_pressed && !esc_pressed) {
                        if state.config.sections.is_empty() {
                            edit.error_msg = Some(s.err_no_sections.to_string());
                        } else {
                            match build_updated_config(&edit, lang) {
                                Ok(new_chk) => {
                                    apply_edit(state, &edit, new_chk);
                                    if state.config.custom_height.is_none() && !state.minimized_to_tray {
                                        ui.ctx().send_viewport_cmd_to(
                                            egui::ViewportId::ROOT,
                                            ViewportCommand::InnerSize(
                                                crate::ui::widget::desired_widget_size(state),
                                            ),
                                        );
                                    }
                                    keep_open = false;
                                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                                }
                                Err(msg) => {
                                    edit.error_msg = Some(msg);
                                }
                            }
                        }
                    }

                    if !edit.is_new && ui.button(s.btn_delete).clicked() {
                        state.delete_check(edit.section_idx, edit.check_idx);
                        if state.config.custom_height.is_none() && !state.minimized_to_tray {
                            ui.ctx().send_viewport_cmd_to(
                                egui::ViewportId::ROOT,
                                ViewportCommand::InnerSize(
                                    crate::ui::widget::desired_widget_size(state),
                                ),
                            );
                        }
                        keep_open = false;
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }

                    if ui.button(s.btn_cancel).clicked() {
                        keep_open = false;
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
        },
    );

    if keep_open {
        state.editing_check = Some(edit);
    }
}

pub fn draw_group(ctx: &egui::Context, state: &mut AppState) {
    let Some(mut grp) = state.editing_group.take() else {
        return;
    };

    let lang = state.config.language;
    let s = tr(lang);
    let title = if grp.section_idx.is_some() {
        s.rename_group_title
    } else {
        s.new_group_title
    };

    let mut builder = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size([340.0, 155.0])
        .with_min_inner_size([300.0, 140.0])
        .with_resizable(false)
        .with_always_on_top();

    let applied_initial = state.edit_open_pos.take();
    if let Some(pos) = applied_initial {
        builder = builder.with_position(pos);
    }

    let mut keep_open = true;
    let is_dark = state.config.theme.resolve_is_dark();

    ctx.show_viewport_immediate(
        ViewportId::from_hash_of("edit_group_dialog"),
        builder,
        |ctx, _class| {
            ctx.set_visuals(if is_dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            track_edit_window_pos(ctx, state, applied_initial.is_some());

            if ctx.input(|i| i.viewport().close_requested()) {
                keep_open = false;
            }
            let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            if esc_pressed {
                keep_open = false;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::new(8.0, 8.0);
                ui.label(s.field_group_name);
                ui.add(egui::TextEdit::singleline(&mut grp.name).desired_width(310.0));

                if let Some(err) = &grp.error_msg {
                    ui.colored_label(Color32::from_rgb(0xFF, 0x6B, 0x6B), err);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(s.btn_save).clicked() || (enter_pressed && !esc_pressed) {
                        let trimmed = grp.name.trim().to_string();
                        if trimmed.is_empty() {
                            grp.error_msg = Some(s.err_enter_group_name.to_string());
                        } else {
                            if let Some(idx) = grp.section_idx {
                                if let Some(sec_cfg) = state.config.sections.get_mut(idx) {
                                    sec_cfg.name = trimmed.clone();
                                }
                                if let Some(sec_st) = state.sections.get_mut(idx) {
                                    sec_st.name = trimmed;
                                }
                            } else {
                                state.config.sections.push(SectionConfig {
                                    name: trimmed.clone(),
                                    checks: Vec::new(),
                                });
                                state.sections.push(SectionState {
                                    name: trimmed,
                                    checks: Vec::new(),
                                });
                            }
                            state.sync_config_to_disk();
                            if state.config.custom_height.is_none() && !state.minimized_to_tray {
                                ui.ctx().send_viewport_cmd_to(
                                    egui::ViewportId::ROOT,
                                    ViewportCommand::InnerSize(
                                        crate::ui::widget::desired_widget_size(state),
                                    ),
                                );
                            }
                            keep_open = false;
                            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                        }
                    }

                    if ui.button(s.btn_cancel).clicked() {
                        keep_open = false;
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
        },
    );

    if keep_open {
        state.editing_group = Some(grp);
    }
}

fn build_updated_config(edit: &EditHostState, lang: Language) -> Result<CheckConfig, String> {
    let s = tr(lang);
    let name = edit.name.trim().to_string();
    if name.is_empty() {
        return Err(s.err_enter_name.to_string());
    }

    let host = edit.host.trim().to_string();
    if host.is_empty() {
        return Err(s.err_enter_host.to_string());
    }

    let port_trim = edit.port_str.trim();
    let port = if port_trim.is_empty() || port_trim == "0" {
        None
    } else {
        let p = port_trim
            .parse::<u16>()
            .map_err(|_| s.err_invalid_port.to_string())?;
        if p == 0 { None } else { Some(p) }
    };

    let timeout_trim = edit.timeout_str.trim();
    let timeout_ms = if timeout_trim.is_empty() {
        2000
    } else {
        timeout_trim
            .parse::<u64>()
            .map_err(|_| s.err_invalid_timeout.to_string())?
            .max(100)
    };

    let int_trim = edit.interval_str.trim();
    let interval_sec = if int_trim.is_empty() || int_trim == "0" {
        None
    } else {
        let v = int_trim
            .parse::<u64>()
            .map_err(|_| s.err_invalid_interval.to_string())?;
        if v == 0 { None } else { Some(v) }
    };

    let kind = if edit.optional {
        format!("{}_optional", edit.base_kind)
    } else {
        edit.base_kind.clone()
    };

    Ok(CheckConfig {
        name,
        host,
        kind,
        port,
        timeout_ms,
        interval_sec,
        optional: false,
        notify: edit.notify,
    })
}

fn apply_edit(state: &mut AppState, edit: &EditHostState, new_chk: CheckConfig) {
    let dest_idx = edit
        .target_section_idx
        .min(state.config.sections.len().saturating_sub(1));

    if edit.is_new {
        if let Some(sec_cfg) = state.config.sections.get_mut(dest_idx) {
            sec_cfg.checks.push(new_chk.clone());
        }
        if let Some(sec_st) = state.sections.get_mut(dest_idx) {
            sec_st.checks.push(CheckState {
                config: new_chk,
                result: None,
                ok_count: 0,
                total_count: 0,
                last_ok_at: None,
            });
        }
        state.sync_config_to_disk();
        state.manual_refresh_requested = true;
        return;
    }

    let valid_src = edit.section_idx < state.config.sections.len()
        && edit.check_idx < state.config.sections[edit.section_idx].checks.len();
    if !valid_src {
        return;
    }

    let old_chk = &state.config.sections[edit.section_idx].checks[edit.check_idx];
    let target_changed = old_chk.host != new_chk.host
        || old_chk.base_kind() != new_chk.base_kind()
        || old_chk.effective_port() != new_chk.effective_port();

    if dest_idx == edit.section_idx {
        state.config.sections[edit.section_idx].checks[edit.check_idx] = new_chk.clone();
        if let Some(ch_state) = state
            .sections
            .get_mut(edit.section_idx)
            .and_then(|s| s.checks.get_mut(edit.check_idx))
        {
            ch_state.config = new_chk;
            if target_changed {
                ch_state.result = None;
                ch_state.ok_count = 0;
                ch_state.total_count = 0;
                ch_state.last_ok_at = None;
            }
        }
    } else {
        state.config.sections[edit.section_idx]
            .checks
            .remove(edit.check_idx);
        state.config.sections[dest_idx]
            .checks
            .push(new_chk.clone());

        let moved_state = if edit.section_idx < state.sections.len()
            && edit.check_idx < state.sections[edit.section_idx].checks.len()
        {
            let mut st = state.sections[edit.section_idx].checks.remove(edit.check_idx);
            st.config = new_chk;
            if target_changed {
                st.result = None;
                st.ok_count = 0;
                st.total_count = 0;
                ch_reset_opt(&mut st);
            }
            st
        } else {
            CheckState {
                config: new_chk,
                result: None,
                ok_count: 0,
                total_count: 0,
                last_ok_at: None,
            }
        };

        if let Some(dest_sec) = state.sections.get_mut(dest_idx) {
            dest_sec.checks.push(moved_state);
        }
    }

    state.sync_config_to_disk();
    state.manual_refresh_requested = true;
}

fn ch_reset_opt(st: &mut CheckState) {
    st.last_ok_at = None;
}
