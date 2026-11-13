#![cfg_attr(all(not(debug_assertions), not(test)), windows_subsystem = "windows")]

mod checks;
mod config;
mod history;
mod i18n;
mod monitor;
mod platform;
mod scheduler;
mod state;
mod tray;
mod ui;
mod updater;

use config::load_config;
use state::{AppState, CheckUpdate};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

struct NetmonApp {
    state: Arc<Mutex<AppState>>,
    rx: UnboundedReceiver<CheckUpdate>,
    trigger_tx: tokio::sync::mpsc::UnboundedSender<()>,
    cfg_tx: tokio::sync::watch::Sender<crate::config::AppConfig>,
    tray_rx: mpsc::Receiver<tray::TrayCommand>,
    update_cmd_tx: tokio::sync::mpsc::UnboundedSender<updater::UpdateCommand>,
    update_evt_rx: UnboundedReceiver<updater::UpdateEvent>,
    last_hide_taskbar: bool,
    initial_phys_pos: Option<[i32; 2]>,
    restarting_for_update: bool,
}

impl eframe::App for NetmonApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        platform::ensure_window_hook();
        if let Some([px, py]) = self.initial_phys_pos.take() {
            platform::set_window_pos_physical(px, py);
        }

        // Проверяем, изменился ли файл .toml на диске (Hot Reload)
        {
            let mut st = self.state.lock().unwrap();
            if st.reload_if_modified() {
                platform::set_edge_snapping(st.config.edge_snapping);
                let _ = self.cfg_tx.send(st.config.clone());
                let _ = self.trigger_tx.send(());
            }
        }

        // Применяем пришедшие результаты проверок
        while let Ok(upd) = self.rx.try_recv() {
            let mut st = self.state.lock().unwrap();
            match upd {
                CheckUpdate::Started => {
                    st.is_checking = true;
                }
                CheckUpdate::Result {
                    section_idx,
                    check_idx,
                    check_key,
                    result,
                } => {
                    st.apply_result_checked(section_idx, check_idx, &check_key, result);
                }
                CheckUpdate::Finished => {
                    st.is_checking = false;
                }
            }
        }

        // Обрабатываем команды из трея
        while let Ok(cmd) = self.tray_rx.try_recv() {
            let mut st = self.state.lock().unwrap();
            match cmd {
                tray::TrayCommand::Menu(act) => {
                    ui::widget::handle_menu_action(act, ui.ctx(), &mut st);
                }
            }
            self.last_hide_taskbar = st.config.hide_taskbar;
        }

        let mut st = self.state.lock().unwrap();
        if st.config.hide_taskbar != self.last_hide_taskbar {
            self.last_hide_taskbar = st.config.hide_taskbar;
            platform::set_skip_taskbar(st.config.hide_taskbar);
        }
        if !st.minimized_to_tray {
            ui::widget::draw(ui, &mut st);
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(1000));
        }

        if st.show_settings {
            ui::settings::draw(ui.ctx(), &mut st);
            self.last_hide_taskbar = st.config.hide_taskbar;
        }
        if st.editing_check.is_some() {
            ui::edit_host::draw(ui.ctx(), &mut st);
        }
        if st.editing_group.is_some() {
            ui::edit_host::draw_group(ui.ctx(), &mut st);
        }

        if st.last_history_save.elapsed().as_secs() >= 30 {
            st.last_history_save = std::time::Instant::now();
            st.uptime_history.save_if_dirty();
        }

        if st.scheduler_needs_sync {
            st.scheduler_needs_sync = false;
            let _ = self.cfg_tx.send(st.config.clone());
        }
        if st.manual_refresh_requested {
            st.manual_refresh_requested = false;
            if st.reload_if_modified() {
                let _ = self.cfg_tx.send(st.config.clone());
            }
            let _ = self.trigger_tx.send(());
        }

        // Обработка событий автообновления (в стиле Firefox)
        while let Ok(evt) = self.update_evt_rx.try_recv() {
            match evt {
                updater::UpdateEvent::CheckedAt(ts) => {
                    st.config.last_update_check_ts = ts;
                    st.save_settings_to_disk();
                }
                updater::UpdateEvent::Status(status) => {
                    st.update_status = status;
                }
                updater::UpdateEvent::NotifyReady(ver) => {
                    let s = crate::i18n::tr(st.config.language);
                    st.pending_notifications.push_back(state::TrayNotification {
                        title: s.notify_update_title.to_string(),
                        body: s.notify_update_body.replace("{}", &ver),
                        level: state::TrayNotifyLevel::Info,
                    });
                }
            }
        }
        if st.manual_update_check_requested {
            st.manual_update_check_requested = false;
            let _ = self
                .update_cmd_tx
                .send(updater::UpdateCommand::CheckNow { manual: true });
        }
        if st.restart_for_update_requested {
            st.restart_for_update_requested = false;
            st.uptime_history.force_save();
            st.save_settings_to_disk();
            match updater::apply_staged_and_restart() {
                Ok(true) => {
                    self.restarting_for_update = true;
                    ui.ctx()
                        .send_viewport_cmd(eframe::egui::ViewportCommand::Close);
                }
                Ok(false) => {}
                Err(e) => {
                    st.update_status = updater::UpdateStatus::Error(e);
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        ui::tooltip::hide();
        if !self.restarting_for_update {
            let mut st = self.state.lock().unwrap();
            st.uptime_history.force_save();
            st.save_settings_to_disk();
        }
    }
}

fn setup_custom_fonts(ctx: &eframe::egui::Context) {
    #[cfg(target_os = "windows")]
    {
        let mut fonts = eframe::egui::FontDefinitions::default();
        let mut loaded = false;

        if let Ok(font_bytes) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
            fonts.font_data.insert(
                "segoe_ui".to_owned(),
                std::sync::Arc::new(eframe::egui::FontData::from_owned(font_bytes)),
            );
            fonts
                .families
                .entry(eframe::egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "segoe_ui".to_owned());
            loaded = true;
        }

        if let Ok(font_bold) = std::fs::read("C:\\Windows\\Fonts\\segoeuib.ttf") {
            fonts.font_data.insert(
                "segoe_ui_bold".to_owned(),
                std::sync::Arc::new(eframe::egui::FontData::from_owned(font_bold)),
            );
        }

        // Подключаем шрифт символов Segoe UI Symbol для корректного отображения иконок/знаков
        if let Ok(sym_bytes) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
            fonts.font_data.insert(
                "segoe_ui_symbol".to_owned(),
                std::sync::Arc::new(eframe::egui::FontData::from_owned(sym_bytes)),
            );
            fonts
                .families
                .entry(eframe::egui::FontFamily::Proportional)
                .or_default()
                .push("segoe_ui_symbol".to_owned());
        }

        if loaded {
            ctx.set_fonts(fonts);
        }
    }
}

fn main() -> eframe::Result<()> {
    updater::cleanup_old_files();
    if matches!(updater::apply_staged_and_restart(), Ok(true)) {
        return Ok(());
    }

    // Токио-рантайм на фоне
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let (tx, rx) = unbounded_channel::<CheckUpdate>();
    let (trigger_tx, trigger_rx) = unbounded_channel::<()>();
    let (tray_tx, tray_rx) = mpsc::channel::<tray::TrayCommand>();

    // Загружаем конфиг и рассчитываем размеры
    let mut cfg = load_config();

    // Высота: шапка (22) + секции (12 каждая) + проверки (17.5 каждая) + рамка
    let calculated_height = 22.0
        + cfg
            .sections
            .iter()
            .map(|s| cfg.size.header_height() + s.checks.len() as f32 * cfg.size.row_height())
            .sum::<f32>()
        + 4.0;
    let initial_width = cfg.custom_width.unwrap_or_else(|| cfg.size.width());
    let initial_height = cfg.custom_height.unwrap_or_else(|| calculated_height.clamp(120.0, 950.0));
    let window_size = eframe::egui::Vec2::new(initial_width, initial_height);

    // Определяем позицию окна (в физических пикселях виртуального экрана) с учетом DPI и прижатия к краю
    let monitors = monitor::get_all_monitors();
    platform::set_edge_snapping(cfg.edge_snapping);
    let snap_thresh = |scale: f32| {
        if cfg.edge_snapping {
            monitor::SNAP_THRESHOLD * scale
        } else {
            0.0
        }
    };
    let (resolved_pos, resolved_mon_id, fallback_active) = if let (Some(saved_pos), Some(saved_id)) = (cfg.window_pos, &cfg.monitor_id) {
        if let Some(target_mon) = monitor::find_monitor(&monitors, saved_id) {
            let phys_size = window_size * target_mon.scale;
            let pos = eframe::egui::Pos2::new(saved_pos[0], saved_pos[1]);
            let snapped = monitor::snap_to_work_area_with_threshold(
                pos,
                phys_size,
                &target_mon.work_rect,
                snap_thresh(target_mon.scale),
                true,
            );
            (snapped, saved_id.clone(), false)
        } else if let Some(fallback_mon) = monitor::primary_or_first(&monitors) {
            let phys_size = window_size * fallback_mon.scale;
            let rel = cfg.rel_pos.unwrap_or([0.9, 0.05]);
            let pos = monitor::pos_from_rel(rel, phys_size, &fallback_mon.work_rect);
            (pos, saved_id.clone(), true)
        } else {
            (eframe::egui::Pos2::new(saved_pos[0], saved_pos[1]), saved_id.clone(), false)
        }
    } else if let Some(saved_pos) = cfg.window_pos {
        let pos = eframe::egui::Pos2::new(saved_pos[0], saved_pos[1]);
        if let Some(mon) = monitor::primary_or_first(&monitors) {
            let phys_size = window_size * mon.scale;
            let snapped = monitor::snap_to_work_area_with_threshold(
                pos,
                phys_size,
                &mon.work_rect,
                snap_thresh(mon.scale),
                true,
            );
            (snapped, mon.id.clone(), false)
        } else {
            (pos, String::new(), false)
        }
    } else if let Some(primary) = monitor::primary_or_first(&monitors) {
        let phys_size = window_size * primary.scale;
        let pos = monitor::pos_from_rel([1.0, 0.05], phys_size, &primary.work_rect);
        (pos, primary.id.clone(), false)
    } else {
        (eframe::egui::Pos2::new(100.0, 100.0), String::new(), false)
    };

    cfg.fallback_active = fallback_active;
    if !fallback_active && !resolved_mon_id.is_empty() {
        cfg.monitor_id = Some(resolved_mon_id);
        cfg.window_pos = Some([resolved_pos.x, resolved_pos.y]);
        if let Some(mon) = monitor::find_monitor(&monitors, cfg.monitor_id.as_deref().unwrap_or("")) {
            let phys_size = window_size * mon.scale;
            cfg.rel_pos = Some(monitor::calculate_rel_pos(resolved_pos, phys_size, &mon.work_rect));
        }
    }

    let (cfg_tx, cfg_rx) = tokio::sync::watch::channel(cfg.clone());
    let cfg_rx_upd = cfg_tx.subscribe();
    let (update_cmd_tx, update_cmd_rx) = unbounded_channel::<updater::UpdateCommand>();
    let (update_evt_tx, update_evt_rx) = unbounded_channel::<updater::UpdateEvent>();

    let egui_ctx_slot: Arc<Mutex<Option<eframe::egui::Context>>> = Arc::new(Mutex::new(None));
    let app_state = Arc::new(Mutex::new(AppState::from_config(&cfg)));
    let _guard = rt.enter();
    scheduler::spawn_scheduler(cfg_rx, tx, trigger_rx, egui_ctx_slot.clone());
    updater::spawn_updater(
        update_cmd_rx,
        cfg_rx_upd,
        update_evt_tx,
        egui_ctx_slot.clone(),
    );

    #[cfg(target_os = "windows")]
    {
        let state_clone = app_state.clone();
        let ctx_slot_clone = egui_ctx_slot.clone();
        tray::spawn_tray(state_clone, tray_tx, ctx_slot_clone);
    }

    let icon_rgba = tray::render_app_icon_rgba(state::OverallStatus::Ok, 64);
    let app_icon = std::sync::Arc::new(eframe::egui::IconData {
        rgba: icon_rgba,
        width: 64,
        height: 64,
    });

    let initial_hide_taskbar = cfg.hide_taskbar;
    let primary_scale = monitor::primary_or_first(&monitors)
        .map(|m| m.scale)
        .unwrap_or(1.0);
    let hint_logical_pos = eframe::egui::Pos2::new(
        resolved_pos.x / primary_scale,
        resolved_pos.y / primary_scale,
    );
    let initial_phys_pos = Some([resolved_pos.x.round() as i32, resolved_pos.y.round() as i32]);
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([initial_width, initial_height])
        .with_min_inner_size([1.0, 1.0])
        .with_position(hint_logical_pos)
        .with_icon(app_icon)
        .with_resizable(true)
        .with_decorations(false)
        .with_transparent(true)
        .with_taskbar(!initial_hide_taskbar)
        .with_mouse_passthrough(false);
    if cfg.always_on_top {
        viewport = viewport.with_always_on_top();
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let state_for_app = app_state.clone();
    eframe::run_native(
        "netmon_widget",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            *egui_ctx_slot.lock().unwrap() = Some(cc.egui_ctx.clone());
            Ok(Box::new(NetmonApp {
                state: state_for_app,
                rx,
                trigger_tx,
                cfg_tx,
                tray_rx,
                update_cmd_tx,
                update_evt_rx,
                last_hide_taskbar: initial_hide_taskbar,
                initial_phys_pos,
                restarting_for_update: false,
            }))
        }),
    )
}
