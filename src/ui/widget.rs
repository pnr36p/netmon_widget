use crate::state::{AppState, CheckState, DragPayload, EditGroupState, EditHostState};
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2,
};

struct Palette {
    bg: Color32,
    border: Color32,
    title_blue: Color32,
    section_bg: Color32,
    section_drag_bg: Color32,
    host_text: Color32,
    row_hover: Color32,
    row_drag: Color32,
    grip: Color32,
    refresh_idle: Color32,
    refresh_hover: Color32,
}

impl Palette {
    fn for_theme(is_dark: bool) -> Self {
        if is_dark {
            Self {
                bg: Color32::from_rgb(0x30, 0x30, 0x30),
                border: Color32::from_rgb(0xAA, 0xAA, 0xAA),
                title_blue: Color32::from_rgb(0x7A, 0xC5, 0xFF),
                section_bg: Color32::from_rgb(0x00, 0x33, 0x99),
                section_drag_bg: Color32::from_rgb(0x16, 0x58, 0xC8),
                host_text: Color32::from_rgb(0xE4, 0xE4, 0xE4),
                row_hover: Color32::from_rgb(0x3C, 0x3C, 0x3C),
                row_drag: Color32::from_rgb(0x46, 0x52, 0x64),
                grip: Color32::from_rgb(0x75, 0x75, 0x75),
                refresh_idle: Color32::from_rgb(0x52, 0x82, 0xC8),
                refresh_hover: Color32::from_rgb(0x9E, 0xC5, 0xFF),
            }
        } else {
            Self {
                bg: Color32::from_rgb(0xF3, 0xF5, 0xF8),
                border: Color32::from_rgb(0xA4, 0xAE, 0xBC),
                title_blue: Color32::from_rgb(0x0F, 0x52, 0xA8),
                section_bg: Color32::from_rgb(0x1E, 0x5B, 0xB8),
                section_drag_bg: Color32::from_rgb(0x34, 0x74, 0xD4),
                host_text: Color32::from_rgb(0x1C, 0x22, 0x29),
                row_hover: Color32::from_rgb(0xE2, 0xE8, 0xF0),
                row_drag: Color32::from_rgb(0xCC, 0xD8, 0xE8),
                grip: Color32::from_rgb(0x8D, 0x96, 0xA3),
                refresh_idle: Color32::from_rgb(0x2B, 0x68, 0xC4),
                refresh_hover: Color32::from_rgb(0x0F, 0x52, 0xA8),
            }
        }
    }
}

pub fn desired_widget_size(state: &AppState) -> Vec2 {
    let sz = state.config.size;
    let w = state.config.custom_width.unwrap_or_else(|| sz.width());
    let calc_h = 22.0
        + state
            .sections
            .iter()
            .map(|sec| sz.header_height() + sec.checks.len() as f32 * sz.row_height())
            .sum::<f32>()
        + 4.0;
    let h = state
        .config
        .custom_height
        .unwrap_or_else(|| calc_h.clamp(120.0, 950.0));
    Vec2::new(w.clamp(110.0, 1000.0), h.clamp(60.0, 2000.0))
}

pub fn apply_minimized_visibility(ctx: &egui::Context, state: &mut AppState) {
    let minimized = state.minimized_to_tray;
    if minimized {
        crate::ui::tooltip::hide();
        crate::platform::set_skip_taskbar(true);
        state.main_outer_rect = None;
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::MousePassthrough(true));
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::InnerSize(Vec2::new(1.0, 1.0)),
        );
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::OuterPosition(Pos2::new(-32000.0, -32000.0)),
        );
    } else {
        let target_size = desired_widget_size(state);
        let monitors = crate::monitor::get_all_monitors();
        let snap_thresh = |scale: f32| {
            if state.config.edge_snapping {
                crate::monitor::SNAP_THRESHOLD * scale
            } else {
                0.0
            }
        };
        let (target_pos, target_scale) = if let (Some(saved_pos), Some(saved_id)) =
            (state.config.window_pos, state.config.monitor_id.as_deref())
        {
            if let Some(mon) = crate::monitor::find_monitor(&monitors, saved_id) {
                let phys_size = target_size * mon.scale;
                let p = crate::monitor::snap_to_work_area_with_threshold(
                    Pos2::new(saved_pos[0], saved_pos[1]),
                    phys_size,
                    &mon.work_rect,
                    snap_thresh(mon.scale),
                    true,
                );
                (p, mon.scale)
            } else if let Some(fb) = crate::monitor::primary_or_first(&monitors) {
                let phys_size = target_size * fb.scale;
                let rel = state.config.rel_pos.unwrap_or([1.0, 0.05]);
                (crate::monitor::pos_from_rel(rel, phys_size, &fb.work_rect), fb.scale)
            } else {
                (Pos2::new(saved_pos[0], saved_pos[1]), 1.0)
            }
        } else if let Some(saved_pos) = state.config.window_pos {
            if let Some(mon) = crate::monitor::primary_or_first(&monitors) {
                let phys_size = target_size * mon.scale;
                let p = crate::monitor::snap_to_work_area_with_threshold(
                    Pos2::new(saved_pos[0], saved_pos[1]),
                    phys_size,
                    &mon.work_rect,
                    snap_thresh(mon.scale),
                    true,
                );
                (p, mon.scale)
            } else {
                (Pos2::new(saved_pos[0], saved_pos[1]), 1.0)
            }
        } else if let Some(primary) = crate::monitor::primary_or_first(&monitors) {
            let phys_size = target_size * primary.scale;
            (
                crate::monitor::pos_from_rel([1.0, 0.05], phys_size, &primary.work_rect),
                primary.scale,
            )
        } else {
            (Pos2::new(100.0, 100.0), 1.0)
        };

        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::MousePassthrough(false));
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::InnerSize(target_size),
        );
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::OuterPosition(Pos2::new(
                target_pos.x / target_scale,
                target_pos.y / target_scale,
            )),
        );
        crate::platform::set_window_pos_physical(
            target_pos.x.round() as i32,
            target_pos.y.round() as i32,
        );
        crate::platform::set_skip_taskbar(state.config.hide_taskbar);
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
    }
}

pub fn handle_menu_action(
    act: crate::ui::menu::native::NativeMenuAction,
    ctx: &egui::Context,
    state: &mut AppState,
) {
    use crate::ui::menu::native::NativeMenuAction;
    match act {
        NativeMenuAction::EditCheck(sec_idx, chk_idx) => {
            if let Some(chk) = state
                .sections
                .get(sec_idx)
                .and_then(|s| s.checks.get(chk_idx))
                .cloned()
            {
                state.open_edit_host(EditHostState::from_check(sec_idx, chk_idx, &chk));
            }
        }
        NativeMenuAction::CloneCheck(sec_idx, chk_idx) => {
            if let Some(chk) = state
                .sections
                .get(sec_idx)
                .and_then(|s| s.checks.get(chk_idx))
                .cloned()
            {
                let suffix = crate::i18n::tr(state.config.language).clone_suffix;
                state.open_edit_host(EditHostState::clone_check(sec_idx, &chk, suffix));
            }
        }
        NativeMenuAction::AddCheck(sec_idx) => {
            let safe_sec = sec_idx.min(state.config.sections.len().saturating_sub(1));
            state.open_edit_host(EditHostState::new_in_section(safe_sec));
        }
        NativeMenuAction::DeleteCheck(sec_idx, chk_idx) => {
            state.delete_check(sec_idx, chk_idx);
            if state.config.custom_height.is_none() && !state.minimized_to_tray {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::InnerSize(desired_widget_size(state)),
                );
            }
        }
        NativeMenuAction::AddGroup => {
            state.open_edit_group(EditGroupState {
                section_idx: None,
                name: String::new(),
                error_msg: None,
            });
        }
        NativeMenuAction::RenameGroup(sec_idx) => {
            if let Some(sec) = state.config.sections.get(sec_idx) {
                state.open_edit_group(EditGroupState {
                    section_idx: Some(sec_idx),
                    name: sec.name.clone(),
                    error_msg: None,
                });
            }
        }
        NativeMenuAction::DeleteGroup(sec_idx) => {
            state.delete_section(sec_idx);
            if state.config.custom_height.is_none() && !state.minimized_to_tray {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::InnerSize(desired_widget_size(state)),
                );
            }
        }
        NativeMenuAction::SetInterval(sec) => {
            state.config.interval_sec = sec.clamp(1, 3600);
            state.save_settings_to_disk();
            state.scheduler_needs_sync = true;
        }
        NativeMenuAction::Refresh => {
            state.manual_refresh_requested = true;
        }
        NativeMenuAction::Settings => {
            state.open_settings();
        }
        NativeMenuAction::ToggleAlwaysOnTop => {
            let aot = !state.config.always_on_top;
            state.config.always_on_top = aot;
            state.save_settings_to_disk();
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::WindowLevel(if aot {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                }),
            );
        }
        NativeMenuAction::ToggleHideTaskbar => {
            state.config.hide_taskbar = !state.config.hide_taskbar;
            state.save_settings_to_disk();
            crate::platform::set_skip_taskbar(state.config.hide_taskbar);
        }
        NativeMenuAction::ToggleMinimize => {
            state.minimized_to_tray = !state.minimized_to_tray;
            apply_minimized_visibility(ctx, state);
        }
        NativeMenuAction::ImportCsv => {
            let path = crate::config::network_csv_path();
            match crate::config::import_network_csv(&path) {
                Ok(sections) => {
                    state.config.sections = sections;
                    state.sync_config_to_disk();
                    let new_cfg = state.config.clone();
                    state.apply_new_config(new_cfg);
                    if state.config.custom_height.is_none() && !state.minimized_to_tray {
                        ctx.send_viewport_cmd_to(
                            egui::ViewportId::ROOT,
                            egui::ViewportCommand::InnerSize(desired_widget_size(state)),
                        );
                    }
                    state.manual_refresh_requested = true;
                }
                Err(e) => {
                    let s = crate::i18n::tr(state.config.language);
                    state.pending_notifications.push_back(crate::state::TrayNotification {
                        title: s.notify_csv_err_title.to_string(),
                        body: e,
                        level: crate::state::TrayNotifyLevel::Warning,
                    });
                }
            }
        }
        NativeMenuAction::EditConfig => {
            let path = crate::config::config_path();
            let _ = std::process::Command::new("notepad.exe").arg(path).spawn();
        }
        NativeMenuAction::RestartToUpdate => {
            state.restart_for_update_requested = true;
        }
        NativeMenuAction::Close => {
            state.save_settings_to_disk();
            state.uptime_history.force_save();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut AppState) {
    let ctx = ui.ctx().clone();
    if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
        if outer.width() > 50.0 {
            state.main_outer_rect = Some(outer);
        }
    }
    let size = state.config.size;
    let fs = size.font_size();
    let header_fs = size.header_font_size();
    let row_height = size.row_height();
    let header_height = size.header_height();

    // Обработка активного перетаскивания границ окна (через Win32 SetWindowPos без джиттера)
    if let Some(mut session) = state.active_resize.take() {
        if ctx.input(|i| i.pointer.primary_down()) {
            crate::platform::apply_window_resize(&mut session, state.config.edge_snapping);
            ctx.set_cursor_icon(session.edge.cursor_icon());
            ctx.request_repaint();
            state.active_resize = Some(session);
        } else if let Some(phys) = crate::platform::get_window_rect_physical() {
            if phys.width() > 20.0 {
                let scale = crate::platform::get_window_dpi_scale().max(0.5);
                state.config.custom_width = Some((phys.width() / scale).clamp(110.0, 1000.0));
                state.config.custom_height = Some((phys.height() / scale).clamp(60.0, 2000.0));
                let monitors = crate::monitor::get_all_monitors();
                if let Some(current_mon) = crate::monitor::best_monitor_for_rect(&monitors, &phys) {
                    state.config.monitor_id = Some(current_mon.id.clone());
                    state.config.window_pos = Some([phys.min.x, phys.min.y]);
                    state.config.rel_pos = Some(crate::monitor::calculate_rel_pos(
                        phys.min,
                        phys.size(),
                        &current_mon.work_rect,
                    ));
                    state.config.fallback_active = false;
                    state.window_pos = Some([phys.min.x, phys.min.y]);
                }
                state.save_settings_to_disk();
            }
        }
    }

    // Периодическая проверка мониторов (раз в секунду)
    let now = std::time::Instant::now();
    if state.active_resize.is_none() && now.duration_since(state.last_monitor_check).as_millis() >= 1000 {
        state.last_monitor_check = now;
        let monitors = crate::monitor::get_all_monitors();

        if state.config.fallback_active {
            // Окно на запасном экране: проверяем, не вернулся ли исходный экран
            if let (Some(saved_id), Some(saved_pos)) = (&state.config.monitor_id, state.config.window_pos) {
                if let Some(orig_mon) = crate::monitor::find_monitor(&monitors, saved_id) {
                    // Исходный экран вернулся! Переносим гаджет на его старое место (физические пиксели)
                    let phys_size = ui.max_rect().size() * orig_mon.scale;
                    let target_pos = Pos2::new(saved_pos[0], saved_pos[1]);
                    let thresh = if state.config.edge_snapping {
                        crate::monitor::SNAP_THRESHOLD * orig_mon.scale
                    } else {
                        0.0
                    };
                    let snapped = crate::monitor::snap_to_work_area_with_threshold(
                        target_pos,
                        phys_size,
                        &orig_mon.work_rect,
                        thresh,
                        true,
                    );
                    crate::platform::set_window_pos_physical(
                        snapped.x.round() as i32,
                        snapped.y.round() as i32,
                    );
                    state.config.fallback_active = false;
                    state.save_settings_to_disk();
                }
            }
        } else {
            // Окно на основном экране: проверяем, не отключился ли он
            if let Some(current_id) = &state.config.monitor_id {
                if crate::monitor::find_monitor(&monitors, current_id).is_none() {
                    // Экран отключился! Переносим на доступный экран
                    if let Some(fallback_mon) = crate::monitor::primary_or_first(&monitors) {
                        let phys_size = ui.max_rect().size() * fallback_mon.scale;
                        let rel = state.config.rel_pos.unwrap_or([0.9, 0.05]);
                        let fallback_pos = crate::monitor::pos_from_rel(
                            rel,
                            phys_size,
                            &fallback_mon.work_rect,
                        );
                        crate::platform::set_window_pos_physical(
                            fallback_pos.x.round() as i32,
                            fallback_pos.y.round() as i32,
                        );
                        state.config.fallback_active = true;
                        state.save_settings_to_disk();
                    }
                }
            }
        }
    }

    // Обработка завершения ручного перетаскивания окна (WM_EXITSIZEMOVE или отпускание мыши)
    let size_move_done = crate::platform::take_size_move_finished();
    if (size_move_done || (state.is_user_dragging_window && !ctx.input(|i| i.pointer.any_down())))
        && state.active_resize.is_none()
    {
        state.is_user_dragging_window = false;

        if let Some(phys) = crate::platform::get_window_rect_physical() {
            if phys.width() > 20.0 {
                let monitors = crate::monitor::get_all_monitors();
                if let Some(current_mon) = crate::monitor::best_monitor_for_rect(&monitors, &phys) {
                    // WM_MOVING уже аккуратно прилепил окно, если edge_snapping включён — просто фиксируем точные координаты
                    state.config.monitor_id = Some(current_mon.id.clone());
                    state.config.window_pos = Some([phys.min.x, phys.min.y]);
                    state.config.rel_pos = Some(crate::monitor::calculate_rel_pos(
                        phys.min,
                        phys.size(),
                        &current_mon.work_rect,
                    ));
                    state.config.fallback_active = false;
                    state.window_pos = Some([phys.min.x, phys.min.y]);
                    state.save_settings_to_disk();
                }
            }
        }
    }

    // Полный прямоугольник гаджета (свободно масштабируемый пользователем)
    let rect = ui.max_rect();
    let width = rect.width();

    let is_dark = state.config.theme.resolve_is_dark();
    let pal = Palette::for_theme(is_dark);

    // Прозрачность фона
    let opacity = state.opacity.clamp(0.2, 1.0);
    let bg_alpha = (255.0 * opacity) as u8;
    let bg = Color32::from_rgba_unmultiplied(pal.bg.r(), pal.bg.g(), pal.bg.b(), bg_alpha);

    // 1. Заливка фона окна
    ui.painter().rect_filled(rect, 0.0, bg);

    // 2. 1px рамка вокруг всего гаджета (характерный стиль Windows Sidebar)
    let border_color =
        Color32::from_rgba_unmultiplied(pal.border.r(), pal.border.g(), pal.border.b(), bg_alpha);
    draw_inside_border(ui.painter(), rect, border_color);

    // Рисуем аккуратную насечку для ресайза в правом нижнем углу
    draw_resize_grip(ui.painter(), rect.right_bottom(), pal.grip);

    // Внутренняя область (без внешних 6px границ ресайза), чтобы клики/drag элементов не конфликтовали с краями
    let safe_interior = Rect::from_min_max(
        Pos2::new(rect.left() + 6.0, rect.top() + 5.0),
        Pos2::new(rect.right() - 6.0, rect.bottom() - 6.0),
    );
    let br_grip_zone = Rect::from_min_max(
        Pos2::new(rect.right() - 14.0, rect.bottom() - 14.0),
        rect.max,
    );
    let press_in_interior = ctx.input(|i| {
        i.pointer
            .press_origin()
            .is_none_or(|p| safe_interior.contains(p) && !br_grip_zone.contains(p))
    });
    let hover_in_interior = ctx.input(|i| {
        i.pointer
            .hover_pos()
            .is_none_or(|p| safe_interior.contains(p) && !br_grip_zone.contains(p))
    });

    let mut current_y = rect.top() + 1.0;

    // ==== 4. Верхняя панель гаджета (22px): слева обновление + заголовок, справа свернуть и закрыть ====
    let bar_height = 22.0;
    let bar_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 1.0, current_y),
        Vec2::new(width - 2.0, bar_height),
    );

    // Кнопка обновления слева: вращающаяся/статичная круговая стрелочка
    let btn_y = (bar_rect.top() + 11.5).max(rect.top() + 12.5);
    let refresh_center = Pos2::new(bar_rect.left() + 13.0, btn_y);
    let refresh_rect = Rect::from_center_size(refresh_center, Vec2::new(15.0, 14.0));
    let refresh_resp = ui.interact(refresh_rect, ui.scope_id().with("gadget_refresh_btn"), Sense::click());
    if refresh_resp.clicked() {
        state.manual_refresh_requested = true;
    }

    // Кнопки управления окном справа: Свернуть (—) и Закрыть (×)
    let close_center = Pos2::new(bar_rect.right() - 12.0, btn_y);
    let close_rect = Rect::from_center_size(close_center, Vec2::new(14.0, 14.0));
    let close_resp = ui.interact(close_rect, ui.scope_id().with("gadget_close_btn"), Sense::click());
    if close_resp.clicked() {
        state.save_settings_to_disk();
        state.uptime_history.force_save();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    let min_center = Pos2::new(bar_rect.right() - 27.0, btn_y);
    let min_rect = Rect::from_center_size(min_center, Vec2::new(14.0, 14.0));
    let min_resp = ui.interact(min_rect, ui.scope_id().with("gadget_minimize_btn"), Sense::click());
    if min_resp.clicked() {
        state.minimized_to_tray = true;
        apply_minimized_visibility(&ctx, state);
    }

    // Перетаскивание окна за свободную область шапки между кнопками (ниже верхних 5px для ресайза)
    let drag_rect = Rect::from_min_max(
        Pos2::new(refresh_rect.right() + 1.0, rect.top() + 5.0),
        Pos2::new((min_rect.left() - 1.0).max(refresh_rect.right() + 2.0), bar_rect.bottom()),
    );
    let bar_resp = ui.interact(drag_rect, ui.scope_id().with("gadget_top_drag"), Sense::click_and_drag());
    if bar_resp.drag_started_by(egui::PointerButton::Primary) && state.active_resize.is_none() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        state.is_user_dragging_window = true;
    }

    let rotation_angle = if state.is_checking {
        ui.input(|i| (i.time * 8.0) as f32)
    } else {
        0.0
    };
    draw_refresh_icon(
        ui.painter(),
        refresh_center,
        4.5,
        rotation_angle,
        if refresh_resp.hovered() {
            pal.refresh_hover
        } else {
            pal.refresh_idle
        },
    );

    // Заголовок после кнопки обновления, обрезается при узком окне перед кнопками справа
    let title_text = truncate_text(
        ui.painter(),
        "Netmon Widget",
        (drag_rect.width() - 4.0).max(8.0),
        11.0,
    );
    ui.painter().with_clip_rect(drag_rect).text(
        Pos2::new(drag_rect.left() + 2.0, bar_rect.center().y),
        Align2::LEFT_CENTER,
        title_text,
        FontId::proportional(11.0),
        pal.title_blue,
    );

    // Иконка "Свернуть в трей" (—)
    let min_color = if min_resp.hovered() {
        pal.refresh_hover
    } else {
        pal.refresh_idle
    };
    ui.painter().line_segment(
        [
            Pos2::new(min_center.x - 3.5, min_center.y + 1.5),
            Pos2::new(min_center.x + 3.5, min_center.y + 1.5),
        ],
        Stroke::new(1.5, min_color),
    );

    // Иконка "Закрыть" (×)
    let close_color = if close_resp.hovered() {
        Color32::from_rgb(0xFF, 0x6B, 0x6B)
    } else {
        pal.refresh_idle
    };
    ui.painter().line_segment(
        [
            Pos2::new(close_center.x - 3.2, close_center.y - 3.2),
            Pos2::new(close_center.x + 3.2, close_center.y + 3.2),
        ],
        Stroke::new(1.5, close_color),
    );
    ui.painter().line_segment(
        [
            Pos2::new(close_center.x + 3.2, close_center.y - 3.2),
            Pos2::new(close_center.x - 3.2, close_center.y + 3.2),
        ],
        Stroke::new(1.5, close_color),
    );

    current_y += bar_height;

    // ==== 5. Секции и хосты ====
    // Скролл на случай, если узлов много и окно меньше контента
    let content_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, current_y),
        Pos2::new(rect.right() - 1.0, rect.bottom() - 1.0),
    );

    let mut content_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );

    let mut hovered_tooltip: Option<(CheckState, Rect)> = None;
    let mut pending_edit: Option<EditHostState> = None;

    enum DragSlot {
        SectionHeader(usize, Rect),
        Check(usize, usize, Rect),
    }

    let mut drag_slots: Vec<DragSlot> = Vec::new();
    let mut section_bounds: Vec<(usize, Rect, Rect)> = Vec::new();
    let primary_down = ctx.input(|i| i.pointer.primary_down());

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(&mut content_ui, |ui| {
            for (sec_idx, section) in state.sections.iter().enumerate() {
                // Синяя полоса-разделитель: перетаскивание изменяет порядок групп в реальном времени
                let (header_rect, header_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), header_height),
                    Sense::click_and_drag(),
                );
                if header_resp.drag_started()
                    && primary_down
                    && state.drag_state.is_none()
                    && state.active_resize.is_none()
                    && press_in_interior
                {
                    state.drag_state = Some(DragPayload::Section { sec_idx });
                }

                let is_dragged_sec = state.drag_state == Some(DragPayload::Section { sec_idx });
                if header_resp.hovered()
                    && state.drag_state.is_none()
                    && state.active_resize.is_none()
                    && hover_in_interior
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }

                ui.painter().rect_filled(
                    header_rect,
                    0.0,
                    if is_dragged_sec {
                        pal.section_drag_bg
                    } else {
                        pal.section_bg
                    },
                );
                if is_dragged_sec {
                    ui.painter().rect_stroke(
                        header_rect,
                        0.0,
                        Stroke::new(1.0, Color32::WHITE),
                        egui::StrokeKind::Inside,
                    );
                }

                // Белый текст строго по центру
                ui.painter().text(
                    header_rect.center(),
                    Align2::CENTER_CENTER,
                    &section.name,
                    FontId::proportional(header_fs),
                    Color32::WHITE,
                );

                drag_slots.push(DragSlot::SectionHeader(sec_idx, header_rect));
                let mut sec_full_rect = header_rect;

                // Строки хостов
                for (chk_idx, check) in section.checks.iter().enumerate() {
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), row_height),
                        Sense::click_and_drag(),
                    );

                    sec_full_rect = sec_full_rect.union(row_rect);
                    drag_slots.push(DragSlot::Check(sec_idx, chk_idx, row_rect));

                    if row_resp.double_clicked() && press_in_interior {
                        pending_edit = Some(EditHostState::from_check(sec_idx, chk_idx, check));
                    } else if row_resp.drag_started()
                        && primary_down
                        && state.drag_state.is_none()
                        && state.active_resize.is_none()
                        && press_in_interior
                    {
                        state.drag_state = Some(DragPayload::Check { sec_idx, chk_idx });
                    }

                    let is_dragged_row =
                        state.drag_state == Some(DragPayload::Check { sec_idx, chk_idx });

                    if is_dragged_row {
                        ui.painter().rect_filled(row_rect, 0.0, pal.row_drag);
                        ui.painter().rect_stroke(
                            row_rect,
                            0.0,
                            Stroke::new(1.0, pal.title_blue),
                            egui::StrokeKind::Inside,
                        );
                    } else if row_resp.hovered()
                        && state.drag_state.is_none()
                        && state.active_resize.is_none()
                        && hover_in_interior
                    {
                        ui.painter().rect_filled(row_rect, 0.0, pal.row_hover);
                        hovered_tooltip = Some((check.clone(), row_rect));
                    }

                    // Имя хоста слева (с отступом x = 9px)
                    let max_text_width = (row_rect.width() - 32.0).max(20.0);
                    let display_name =
                        truncate_text(ui.painter(), &check.config.name, max_text_width, fs);

                    ui.painter().text(
                        Pos2::new(row_rect.left() + 9.0, row_rect.center().y),
                        Align2::LEFT_CENTER,
                        display_name,
                        FontId::proportional(fs),
                        pal.host_text,
                    );

                    // 3D-иконка статуса справа
                    let icon_center = Pos2::new(row_rect.right() - 13.0, row_rect.center().y);
                    match &check.result {
                        Some(res) if res.ok => {
                            draw_glossy_checkmark(ui.painter(), icon_center);
                        }
                        Some(_res) if check.config.is_optional() => {
                            draw_glossy_optional_offline(ui.painter(), icon_center);
                        }
                        Some(_res) => {
                            draw_glossy_cross(ui.painter(), icon_center);
                        }
                        None => {
                            draw_pending_icon(ui.painter(), icon_center);
                        }
                    }
                }

                section_bounds.push((sec_idx, header_rect, sec_full_rect));
            }
        });

    // Обработка живого перетаскивания элементов и групп в реальном времени
    if let Some(drag) = state.drag_state {
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        if primary_down {
            if let Some(ptr) = ctx.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos())) {
                let py = ptr.y;
                match drag {
                    DragPayload::Check { sec_idx, chk_idx } => {
                        if let Some(pos) = drag_slots.iter().position(|slot| {
                            matches!(slot, DragSlot::Check(s, c, _) if *s == sec_idx && *c == chk_idx)
                        }) {
                            let slot_rect = |s: &DragSlot| match s {
                                DragSlot::SectionHeader(_, r) | DragSlot::Check(_, _, r) => *r,
                            };
                            if pos > 0 && py < slot_rect(&drag_slots[pos - 1]).center().y {
                                let target = match drag_slots[pos - 1] {
                                    DragSlot::Check(ps, pc, _) => Some((ps, pc)),
                                    DragSlot::SectionHeader(hs, _) if hs > 0 => {
                                        Some((hs - 1, state.sections[hs - 1].checks.len()))
                                    }
                                    _ => None,
                                };
                                if let Some(to) = target {
                                    let (ns, nc) = state.move_check((sec_idx, chk_idx), to);
                                    state.drag_state = Some(DragPayload::Check {
                                        sec_idx: ns,
                                        chk_idx: nc,
                                    });
                                }
                            } else if pos + 1 < drag_slots.len()
                                && py > slot_rect(&drag_slots[pos + 1]).center().y
                            {
                                let target = match drag_slots[pos + 1] {
                                    DragSlot::Check(ns, nc, _) => Some((ns, nc)),
                                    DragSlot::SectionHeader(hs, _) => Some((hs, 0)),
                                };
                                if let Some(to) = target {
                                    let (ns, nc) = state.move_check((sec_idx, chk_idx), to);
                                    state.drag_state = Some(DragPayload::Check {
                                        sec_idx: ns,
                                        chk_idx: nc,
                                    });
                                }
                            }
                        }
                    }
                    DragPayload::Section { sec_idx } => {
                        if sec_idx > 0 && py < section_bounds[sec_idx - 1].1.bottom() {
                            state.swap_sections(sec_idx, sec_idx - 1);
                            state.drag_state = Some(DragPayload::Section {
                                sec_idx: sec_idx - 1,
                            });
                        } else if sec_idx + 1 < section_bounds.len()
                            && py > section_bounds[sec_idx + 1].2.bottom() - header_height
                        {
                            state.swap_sections(sec_idx, sec_idx + 1);
                            state.drag_state = Some(DragPayload::Section {
                                sec_idx: sec_idx + 1,
                            });
                        }
                    }
                }
            }
        } else {
            state.drag_state = None;
            state.sync_config_to_disk();
        }
    }

    if let Some(edit) = pending_edit {
        state.open_edit_host(edit);
        crate::ui::tooltip::hide();
    }

    // Интерактивные зоны изменения размера по ВСЕМ 4 сторонам и 4 углам (поверх контента)
    {
        use crate::platform::ResizeEdge;
        let edge = 6.0;
        let top_h = 5.0;
        let top_corner = 8.0;
        let corner = 10.0;
        let br = 14.0;
        let handles = [
            (
                ResizeEdge::TopLeft,
                Rect::from_min_max(rect.min, Pos2::new(rect.left() + top_corner, rect.top() + top_corner)),
                "res_tl",
            ),
            (
                ResizeEdge::TopRight,
                Rect::from_min_max(
                    Pos2::new(rect.right() - top_corner, rect.top()),
                    Pos2::new(rect.right(), rect.top() + top_corner),
                ),
                "res_tr",
            ),
            (
                ResizeEdge::BottomLeft,
                Rect::from_min_max(
                    Pos2::new(rect.left(), rect.bottom() - corner),
                    Pos2::new(rect.left() + corner, rect.bottom()),
                ),
                "res_bl",
            ),
            (
                ResizeEdge::BottomRight,
                Rect::from_min_max(Pos2::new(rect.right() - br, rect.bottom() - br), rect.max),
                "res_br",
            ),
            (
                ResizeEdge::Left,
                Rect::from_min_max(
                    Pos2::new(rect.left(), rect.top() + top_corner),
                    Pos2::new(rect.left() + edge, rect.bottom() - corner),
                ),
                "res_l",
            ),
            (
                ResizeEdge::Right,
                Rect::from_min_max(
                    Pos2::new(rect.right() - edge, rect.top() + top_corner),
                    Pos2::new(rect.right(), rect.bottom() - corner),
                ),
                "res_r",
            ),
            (
                ResizeEdge::Top,
                Rect::from_min_max(
                    Pos2::new(rect.left() + top_corner, rect.top()),
                    Pos2::new(rect.right() - top_corner, rect.top() + top_h),
                ),
                "res_t",
            ),
            (
                ResizeEdge::Bottom,
                Rect::from_min_max(
                    Pos2::new(rect.left() + corner, rect.bottom() - edge),
                    Pos2::new(rect.right() - br, rect.bottom()),
                ),
                "res_b",
            ),
        ];

        for (edge_kind, hrect, salt) in handles {
            let resp = ui.interact(hrect, ui.scope_id().with(salt), Sense::click_and_drag());
            if state.drag_state.is_none() && (resp.hovered() || resp.dragged()) {
                ctx.set_cursor_icon(edge_kind.cursor_icon());
            }
            if state.active_resize.is_none()
                && state.drag_state.is_none()
                && resp.drag_started_by(egui::PointerButton::Primary)
            {
                if let Some(session) = crate::platform::begin_window_resize(edge_kind) {
                    state.active_resize = Some(session);
                    ctx.set_cursor_icon(edge_kind.cursor_icon());
                    ctx.request_repaint();
                }
            }
        }
    }

    // Нативное Win32 контекстное меню по правому клику (с определением строки / группы под курсором)
    let secondary_clicked = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary));
    if secondary_clicked {
        let click_pos = ctx.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));
        if let Some(pos) = click_pos {
            if rect.contains(pos) {
                crate::ui::tooltip::hide();
                #[cfg(target_os = "windows")]
                {
                    let hit_check = drag_slots.iter().find_map(|s| match s {
                        DragSlot::Check(si, ci, r) if r.contains(pos) => Some((*si, *ci)),
                        _ => None,
                    });
                    let hit_section = drag_slots.iter().find_map(|s| match s {
                        DragSlot::SectionHeader(si, r) if r.contains(pos) => Some(*si),
                        _ => None,
                    });

                    let menu_opts = crate::ui::menu::native::PopupMenuState {
                        lang: state.config.language,
                        is_dark,
                        always_on_top: state.config.always_on_top,
                        minimized: state.minimized_to_tray,
                        hide_taskbar: state.config.hide_taskbar,
                        update_ready: matches!(
                            state.update_status,
                            crate::updater::UpdateStatus::ReadyToRestart(_)
                        ),
                        interval_sec: state.config.interval_sec,
                    };
                    let action = if let Some((si, ci)) = hit_check {
                        let name = state.sections[si].checks[ci].config.name.clone();
                        let sec_name = state.sections[si].name.clone();
                        crate::ui::menu::native::show_popup(
                            menu_opts,
                            crate::ui::menu::native::MenuTarget::Check {
                                sec_idx: si,
                                chk_idx: ci,
                                name: &name,
                                sec_name: &sec_name,
                            },
                        )
                    } else if let Some(si) = hit_section {
                        let name = state.sections[si].name.clone();
                        crate::ui::menu::native::show_popup(
                            menu_opts,
                            crate::ui::menu::native::MenuTarget::Section {
                                sec_idx: si,
                                name: &name,
                            },
                        )
                    } else {
                        crate::ui::menu::native::show_popup(
                            menu_opts,
                            crate::ui::menu::native::MenuTarget::EmptyArea,
                        )
                    };

                    if let Some(act) = action {
                        handle_menu_action(act, &ctx, state);
                    }
                }
            }
        }
    }

    // Плавающая подсказка за пределами окна гаджета
    let can_show_tooltip = !state.is_user_dragging_window
        && state.active_resize.is_none()
        && state.drag_state.is_none()
        && state.editing_check.is_none()
        && state.editing_group.is_none()
        && ctx.input(|i| i.pointer.has_pointer());
    if can_show_tooltip {
        if let (Some((check, row_rect)), Some(outer)) = (
            hovered_tooltip,
            ctx.input(|i| i.viewport().outer_rect),
        ) {
            let now_ts = chrono::Utc::now().timestamp();
            let windows = state.uptime_history.windows(&check.config, now_ts);
            crate::ui::tooltip::show(
                &check,
                windows,
                state.config.language,
                is_dark,
                row_rect,
                outer,
                ctx.pixels_per_point(),
            );
        } else {
            crate::ui::tooltip::hide();
        }
    } else {
        crate::ui::tooltip::hide();
    }

    // Регулярная перерисовка (для анимации вращения и таймеров)
    if state.is_checking {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    } else {
        ctx.request_repaint_after(std::time::Duration::from_millis(1000));
    }
}

// ============================================================================
// Векторная отрисовка иконок Windows 7 Gadget
// ============================================================================

/// Рисует внутреннюю 1px рамку окна
fn draw_inside_border(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.0, color);
    let p_min = rect.min + Vec2::new(0.5, 0.5);
    let p_max = rect.max - Vec2::new(0.5, 0.5);

    painter.line_segment([Pos2::new(p_min.x, p_min.y), Pos2::new(p_max.x, p_min.y)], stroke);
    painter.line_segment([Pos2::new(p_max.x, p_min.y), Pos2::new(p_max.x, p_max.y)], stroke);
    painter.line_segment([Pos2::new(p_max.x, p_max.y), Pos2::new(p_min.x, p_max.y)], stroke);
    painter.line_segment([Pos2::new(p_min.x, p_max.y), Pos2::new(p_min.x, p_min.y)], stroke);
}

/// Ненавязчивая классическая насечка изменения размера в правом нижнем углу
fn draw_resize_grip(painter: &egui::Painter, corner: Pos2, color: Color32) {
    let pts = [
        corner - Vec2::new(4.0, 4.0),
        corner - Vec2::new(8.0, 4.0),
        corner - Vec2::new(4.0, 8.0),
        corner - Vec2::new(12.0, 4.0),
        corner - Vec2::new(8.0, 8.0),
        corner - Vec2::new(4.0, 12.0),
    ];
    for p in pts {
        painter.circle_filled(p, 1.0, color);
    }
}

/// Глянцевая объёмная зелёная галочка ✔ (точно как на скриншоте)
fn draw_glossy_checkmark(painter: &egui::Painter, center: Pos2) {
    let cx = center.x;
    let cy = center.y;

    // Координаты вершин галочки
    let p1 = Pos2::new(cx - 4.2, cy + 0.1);
    let p2 = Pos2::new(cx - 1.5, cy + 3.6);
    let p3 = Pos2::new(cx + 4.2, cy - 3.8);

    // 1. Тёмно-зелёная тень/фаска снизу-справа (3D bevel)
    let shadow_color = Color32::from_rgb(0x1B, 0x6E, 0x22);
    let p1_s = p1 + Vec2::new(0.8, 0.8);
    let p2_s = p2 + Vec2::new(0.8, 0.8);
    let p3_s = p3 + Vec2::new(0.8, 0.8);
    painter.line_segment([p1_s, p2_s], Stroke::new(2.8, shadow_color));
    painter.line_segment([p2_s, p3_s], Stroke::new(2.8, shadow_color));

    // 2. Основное тело: сочный ярко-зеленый #32D73C
    let main_color = Color32::from_rgb(0x32, 0xD7, 0x3C);
    painter.line_segment([p1, p2], Stroke::new(2.4, main_color));
    painter.line_segment([p2, p3], Stroke::new(2.4, main_color));

    // 3. Светлый блик (gloss) по верху длинного хвостика
    let gloss_color = Color32::from_rgb(0x9E, 0xF8, 0xA4);
    let p_g1 = Pos2::new(cx - 0.8, cy + 2.0);
    let p_g2 = Pos2::new(cx + 3.8, cy - 3.4);
    painter.line_segment([p_g1, p_g2], Stroke::new(1.0, gloss_color));
}

/// Глянцевый объёмный красный крестик ✖ (точно как на скриншоте у Canon MFD)
fn draw_glossy_cross(painter: &egui::Painter, center: Pos2) {
    let cx = center.x;
    let cy = center.y;

    let p1_a = Pos2::new(cx - 3.8, cy - 3.8);
    let p1_b = Pos2::new(cx + 3.6, cy + 3.6);
    let p2_a = Pos2::new(cx + 3.6, cy - 3.8);
    let p2_b = Pos2::new(cx - 3.8, cy + 3.6);

    // 1. Тёмно-красная тень/фаска снизу-справа
    let shadow_color = Color32::from_rgb(0x8A, 0x12, 0x12);
    let off = Vec2::new(0.8, 0.8);
    painter.line_segment([p1_a + off, p1_b + off], Stroke::new(2.8, shadow_color));
    painter.line_segment([p2_a + off, p2_b + off], Stroke::new(2.8, shadow_color));

    // 2. Ярко-красное тело #FA2424
    let main_color = Color32::from_rgb(0xFA, 0x24, 0x24);
    painter.line_segment([p1_a, p1_b], Stroke::new(2.4, main_color));
    painter.line_segment([p2_a, p2_b], Stroke::new(2.4, main_color));

    // 3. Блик по диагонали
    let gloss_color = Color32::from_rgb(0xFF, 0x94, 0x94);
    painter.line_segment(
        [Pos2::new(cx - 2.8, cy - 2.8), Pos2::new(cx + 2.6, cy + 2.6)],
        Stroke::new(1.0, gloss_color),
    );
}

/// Глянцевый объёмный жёлтый минус — для некритичных отключённых устройств (например, принтера)
fn draw_glossy_optional_offline(painter: &egui::Painter, center: Pos2) {
    let cx = center.x;
    let cy = center.y;

    let p_left = Pos2::new(cx - 4.0, cy);
    let p_right = Pos2::new(cx + 4.0, cy);

    // 1. Тёмно-янтарная тень/фаска снизу-справа
    let shadow_color = Color32::from_rgb(0x8A, 0x62, 0x00);
    let off = Vec2::new(0.8, 0.8);
    painter.line_segment([p_left + off, p_right + off], Stroke::new(3.0, shadow_color));

    // 2. Основное тело: сочный янтарно-жёлтый #FFCA18
    let main_color = Color32::from_rgb(0xFF, 0xCA, 0x18);
    painter.line_segment([p_left, p_right], Stroke::new(2.6, main_color));

    // 3. Светлый блик сверху
    let gloss_color = Color32::from_rgb(0xFF, 0xF2, 0x99);
    painter.line_segment(
        [Pos2::new(cx - 3.2, cy - 0.6), Pos2::new(cx + 3.2, cy - 0.6)],
        Stroke::new(0.9, gloss_color),
    );
}

/// Иконка ожидания (pending)
fn draw_pending_icon(painter: &egui::Painter, center: Pos2) {
    let stroke = Stroke::new(1.2, Color32::from_rgb(0x88, 0x88, 0x88));
    painter.circle_stroke(center, 3.5, stroke);
    painter.circle_filled(center, 1.2, Color32::from_rgb(0xC0, 0xC0, 0xC0));
}

/// Круговые синие стрелочки обновления 🗘
fn draw_refresh_icon(painter: &egui::Painter, center: Pos2, r: f32, angle: f32, color: Color32) {
    let stroke = Stroke::new(1.5, color);

    // Верхняя дуга + стрелка
    draw_arc_with_arrow(painter, center, r, angle, 2.3, stroke, color);
    // Нижняя дуга + стрелка
    draw_arc_with_arrow(
        painter,
        center,
        r,
        angle + std::f32::consts::PI,
        2.3,
        stroke,
        color,
    );
}

fn draw_arc_with_arrow(
    painter: &egui::Painter,
    center: Pos2,
    r: f32,
    start_angle: f32,
    arc_len: f32,
    stroke: Stroke,
    color: Color32,
) {
    let steps = 7;
    for i in 0..steps {
        let a1 = start_angle + (i as f32 / steps as f32) * arc_len;
        let a2 = start_angle + ((i + 1) as f32 / steps as f32) * arc_len;
        painter.line_segment(
            [
                Pos2::new(center.x + r * a1.cos(), center.y + r * a1.sin()),
                Pos2::new(center.x + r * a2.cos(), center.y + r * a2.sin()),
            ],
            stroke,
        );
    }

    // Наконечник стрелки на конце дуги
    let tip_angle = start_angle + arc_len;
    let tip = Pos2::new(center.x + r * tip_angle.cos(), center.y + r * tip_angle.sin());
    let tang = tip_angle + std::f32::consts::FRAC_PI_2;
    let barb1 = tip + Vec2::new((tang + 2.4).cos(), (tang + 2.4).sin()) * 2.6;
    let barb2 = tip + Vec2::new((tang - 2.4).cos(), (tang - 2.4).sin()) * 2.6;
    painter.line_segment([tip, barb1], Stroke::new(1.3, color));
    painter.line_segment([tip, barb2], Stroke::new(1.3, color));
}

/// Обрезает длинный текст с добавлением многоточия по фактической ширине глифов
fn truncate_text(painter: &egui::Painter, text: &str, max_width_px: f32, font_size: f32) -> String {
    if text.is_empty() {
        return String::new();
    }
    let font_id = FontId::proportional(font_size);
    if painter
        .layout_no_wrap(text.to_string(), font_id.clone(), egui::Color32::WHITE)
        .size()
        .x
        <= max_width_px
    {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let mut candidate: String = chars[..mid].iter().collect();
        candidate.push('…');
        if painter
            .layout_no_wrap(candidate, font_id.clone(), egui::Color32::WHITE)
            .size()
            .x
            <= max_width_px
        {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 {
        '…'.to_string()
    } else {
        let mut s: String = chars[..lo].iter().collect();
        s.push('…');
        s
    }
}
