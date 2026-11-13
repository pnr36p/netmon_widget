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
    last_hide_taskbar: bool,
}

impl NetmonApp {
    fn new(
        rx: UnboundedReceiver<CheckUpdate>,
        trigger_tx: tokio::sync::mpsc::UnboundedSender<()>,
        cfg_tx: tokio::sync::watch::Sender<crate::config::AppConfig>,
        tray_rx: mpsc::Receiver<tray::TrayCommand>,
        app_state: Arc<Mutex<AppState>>,
        initial_hide_taskbar: bool,
    ) -> Self {
        Self {
            state: app_state,
            rx,
            trigger_tx,
            cfg_tx,
            tray_rx,
            last_hide_taskbar: initial_hide_taskbar,
        }
    }
}

impl eframe::App for NetmonApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        // Проверяем, изменился ли файл .toml на диске (Hot Reload)
        {
            let mut st = self.state.lock().unwrap();
            if st.reload_if_modified() {
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
                    result,
                } => {
                    st.apply_result(section_idx, check_idx, result);
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
                .request_repaint_after(std::time::Duration::from_millis(250));
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
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        ui::tooltip::hide();
        let mut st = self.state.lock().unwrap();
        st.uptime_history.force_save();
        crate::config::save_config(&st.config);
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
            .map(|s| 12.0 + s.checks.len() as f32 * 17.5)
            .sum::<f32>()
        + 4.0;
    let initial_width = cfg.custom_width.unwrap_or_else(|| cfg.size.width());
    let initial_height = cfg.custom_height.unwrap_or_else(|| calculated_height.clamp(120.0, 950.0));
    let window_size = eframe::egui::Vec2::new(initial_width, initial_height);

    // Определяем позицию окна с учетом мониторов, fallback и прижатия к краю
    let monitors = monitor::get_all_monitors();
    let (resolved_pos, resolved_mon_id, fallback_active) = if let (Some(saved_pos), Some(saved_id)) = (cfg.window_pos, &cfg.monitor_id) {
        if let Some(target_mon) = monitor::find_monitor(&monitors, saved_id) {
            // Исходный экран подключен: восстанавливаем позицию с прижатием к краю
            let pos = eframe::egui::Pos2::new(saved_pos[0], saved_pos[1]);
            let snapped = monitor::snap_to_work_area(pos, window_size, &target_mon.work_rect);
            (snapped, saved_id.clone(), false)
        } else if let Some(fallback_mon) = monitor::primary_or_first(&monitors) {
            // Исходный экран отсутствует: отображаем в относительно той же позиции на доступном экране
            let rel = cfg.rel_pos.unwrap_or([0.9, 0.05]);
            let pos = monitor::pos_from_rel(rel, window_size, &fallback_mon.work_rect);
            (pos, saved_id.clone(), true)
        } else {
            (eframe::egui::Pos2::new(saved_pos[0], saved_pos[1]), saved_id.clone(), false)
        }
    } else if let Some(saved_pos) = cfg.window_pos {
        let pos = eframe::egui::Pos2::new(saved_pos[0], saved_pos[1]);
        if let Some(mon) = monitor::primary_or_first(&monitors) {
            let snapped = monitor::snap_to_work_area(pos, window_size, &mon.work_rect);
            (snapped, mon.id.clone(), false)
        } else {
            (pos, String::new(), false)
        }
    } else if let Some(primary) = monitor::primary_or_first(&monitors) {
        // Первый запуск: прижимаем к правому краю основного экрана (классический стиль Windows 7 Sidebar)
        let pos = monitor::pos_from_rel([1.0, 0.05], window_size, &primary.work_rect);
        (pos, primary.id.clone(), false)
    } else {
        (eframe::egui::Pos2::new(100.0, 100.0), String::new(), false)
    };

    cfg.fallback_active = fallback_active;
    if !fallback_active && !resolved_mon_id.is_empty() {
        cfg.monitor_id = Some(resolved_mon_id);
        cfg.window_pos = Some([resolved_pos.x, resolved_pos.y]);
        if let Some(mon) = monitor::find_monitor(&monitors, cfg.monitor_id.as_deref().unwrap_or("")) {
            cfg.rel_pos = Some(monitor::calculate_rel_pos(resolved_pos, window_size, &mon.work_rect));
        }
    }

    let (cfg_tx, cfg_rx) = tokio::sync::watch::channel(cfg.clone());
    let app_state = Arc::new(Mutex::new(AppState::from_config(&cfg)));
    let _guard = rt.enter();
    scheduler::spawn_scheduler(cfg_rx, tx, trigger_rx);

    let egui_ctx_slot: Arc<Mutex<Option<eframe::egui::Context>>> = Arc::new(Mutex::new(None));

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
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([initial_width, initial_height])
        .with_min_inner_size([1.0, 1.0])
        .with_position(resolved_pos)
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
            Ok(Box::new(NetmonApp::new(
                rx,
                trigger_tx,
                cfg_tx,
                tray_rx,
                state_for_app,
                initial_hide_taskbar,
            )))
        }),
    )
}
