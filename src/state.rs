use crate::config::{AppConfig, CheckConfig, SectionConfig};
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct CheckState {
    pub config: CheckConfig,
    pub result: Option<CheckResult>,
    pub ok_count: u32,
    pub total_count: u32,
    pub last_ok_at: Option<DateTime<Local>>,
}

#[derive(Debug, Clone)]
pub struct SectionState {
    pub name: String,
    pub checks: Vec<CheckState>,
}

#[derive(Debug, Clone)]
pub struct EditHostState {
    pub is_new: bool,
    pub section_idx: usize,
    pub check_idx: usize,
    pub target_section_idx: usize,
    pub name: String,
    pub host: String,
    pub base_kind: String,
    pub optional: bool,
    pub notify: bool,
    pub port_str: String,
    pub timeout_str: String,
    pub interval_str: String,
    pub error_msg: Option<String>,
}

impl EditHostState {
    pub fn from_check(section_idx: usize, check_idx: usize, check: &CheckState) -> Self {
        Self {
            is_new: false,
            section_idx,
            check_idx,
            target_section_idx: section_idx,
            name: check.config.name.clone(),
            host: check.config.host.clone(),
            base_kind: check.config.base_kind().to_string(),
            optional: check.config.is_optional(),
            notify: check.config.notify,
            port_str: check.config.port.map(|p| p.to_string()).unwrap_or_default(),
            timeout_str: check.config.timeout_ms.to_string(),
            interval_str: check
                .config
                .interval_sec
                .map(|v| v.to_string())
                .unwrap_or_default(),
            error_msg: None,
        }
    }

    pub fn clone_check(section_idx: usize, check: &CheckState, copy_suffix: &str) -> Self {
        let mut cloned = Self::from_check(section_idx, 0, check);
        cloned.is_new = true;
        cloned.name = format!("{}{}", check.config.name, copy_suffix);
        cloned
    }

    pub fn new_in_section(section_idx: usize) -> Self {
        Self {
            is_new: true,
            section_idx,
            check_idx: 0,
            target_section_idx: section_idx,
            name: String::new(),
            host: String::new(),
            base_kind: "ping".to_string(),
            optional: false,
            notify: false,
            port_str: String::new(),
            timeout_str: "2000".to_string(),
            interval_str: String::new(),
            error_msg: None,
        }
    }

    pub fn default_port_hint(&self, lang: crate::i18n::Language) -> &'static str {
        let s = crate::i18n::tr(lang);
        match self.base_kind.as_str() {
            "https" => s.hint_port_443,
            "http" | "tcp" => s.hint_port_80,
            "dns" => s.hint_port_53,
            _ => s.hint_port_none,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditGroupState {
    /// `None` — создание новой группы, `Some(idx)` — переименование существующей.
    pub section_idx: Option<usize>,
    pub name: String,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPayload {
    Check { sec_idx: usize, chk_idx: usize },
    Section { sec_idx: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayNotifyLevel {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayNotification {
    pub title: String,
    pub body: String,
    pub level: TrayNotifyLevel,
}

pub struct AppState {
    pub config: AppConfig,
    pub sections: Vec<SectionState>,
    pub opacity: f32,
    pub show_settings: bool,
    pub settings_open_pos: Option<eframe::egui::Pos2>,
    pub editing_check: Option<EditHostState>,
    pub edit_open_pos: Option<eframe::egui::Pos2>,
    pub editing_group: Option<EditGroupState>,
    pub drag_state: Option<DragPayload>,
    pub main_outer_rect: Option<eframe::egui::Rect>,
    pub window_pos: Option<[f32; 2]>,
    pub is_checking: bool,
    pub manual_refresh_requested: bool,
    pub scheduler_needs_sync: bool,
    pub config_modified: Option<std::time::SystemTime>,
    pub active_resize: Option<crate::platform::WindowResizeSession>,
    pub is_user_dragging_window: bool,
    pub last_monitor_check: std::time::Instant,
    pub uptime_history: crate::history::UptimeHistory,
    pub last_history_save: std::time::Instant,
    pub autostart_enabled: bool,
    pub autostart_error: Option<String>,
    pub minimized_to_tray: bool,
    pub update_status: crate::updater::UpdateStatus,
    pub manual_update_check_requested: bool,
    pub restart_for_update_requested: bool,
    pub pending_notifications: std::collections::VecDeque<TrayNotification>,
}

impl AppState {
    pub fn from_config(cfg: &AppConfig) -> Self {
        let sections = cfg
            .sections
            .iter()
            .map(|s: &SectionConfig| SectionState {
                name: s.name.clone(),
                checks: s
                    .checks
                    .iter()
                    .map(|c| CheckState {
                        config: c.clone(),
                        result: None,
                        ok_count: 0,
                        total_count: 0,
                        last_ok_at: None,
                    })
                    .collect(),
            })
            .collect();

        let config_modified = std::fs::metadata(crate::config::config_path())
            .and_then(|m| m.modified())
            .ok();

        Self {
            opacity: cfg.opacity,
            config: cfg.clone(),
            sections,
            show_settings: false,
            settings_open_pos: None,
            editing_check: None,
            edit_open_pos: None,
            editing_group: None,
            drag_state: None,
            main_outer_rect: None,
            window_pos: cfg.window_pos,
            is_checking: false,
            manual_refresh_requested: false,
            scheduler_needs_sync: false,
            config_modified,
            active_resize: None,
            is_user_dragging_window: false,
            last_monitor_check: std::time::Instant::now(),
            uptime_history: crate::history::UptimeHistory::load(),
            last_history_save: std::time::Instant::now(),
            autostart_enabled: crate::platform::is_autostart_enabled(),
            autostart_error: None,
            minimized_to_tray: false,
            update_status: crate::updater::UpdateStatus::Idle,
            manual_update_check_requested: false,
            restart_for_update_requested: false,
            pending_notifications: std::collections::VecDeque::new(),
        }
    }

    /// Проверяет, изменился ли файл .toml на диске, и при необходимости перезагружает хосты.
    pub fn reload_if_modified(&mut self) -> bool {
        let path = crate::config::config_path();
        self.reload_from_path_if_modified(&path)
    }

    pub fn reload_from_path_if_modified(&mut self, path: &std::path::Path) -> bool {
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                if self.config_modified.is_none() {
                    self.config_modified = Some(mtime);
                    return false;
                }
                if Some(mtime) != self.config_modified {
                    self.config_modified = Some(mtime);
                    if let Ok(mut new_cfg) = crate::config::try_load_config_from(path) {
                        if new_cfg.sections.is_empty() && !self.config.sections.is_empty() {
                            new_cfg.sections = self.config.sections.clone();
                        }
                        self.apply_new_config(new_cfg);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Применяет новый конфиг, сохраняя уже полученные статусы проверок для существующих хостов.
    pub fn apply_new_config(&mut self, new_cfg: AppConfig) {
        self.config = new_cfg.clone();
        self.opacity = new_cfg.opacity;

        let mut new_sections = Vec::new();
        for s in &new_cfg.sections {
            let mut checks = Vec::new();
            for c in &s.checks {
                let existing = self
                    .sections
                    .iter()
                    .flat_map(|sec| sec.checks.iter())
                    .find(|ch| {
                        ch.config.name == c.name
                            && ch.config.host == c.host
                            && ch.config.base_kind() == c.base_kind()
                            && ch.config.effective_port() == c.effective_port()
                    })
                    .or_else(|| {
                        self.sections
                            .iter()
                            .flat_map(|sec| sec.checks.iter())
                            .find(|ch| {
                                ch.config.host == c.host
                                    && ch.config.base_kind() == c.base_kind()
                                    && ch.config.effective_port() == c.effective_port()
                            })
                    });

                checks.push(CheckState {
                    config: c.clone(),
                    result: existing.and_then(|ch| ch.result.clone()),
                    ok_count: existing.map(|ch| ch.ok_count).unwrap_or(0),
                    total_count: existing.map(|ch| ch.total_count).unwrap_or(0),
                    last_ok_at: existing.and_then(|ch| ch.last_ok_at),
                });
            }
            new_sections.push(SectionState {
                name: s.name.clone(),
                checks,
            });
        }
        self.sections = new_sections;
    }

    pub fn apply_result_checked(
        &mut self,
        section_idx: usize,
        check_idx: usize,
        check_key: &str,
        res: CheckResult,
    ) {
        let matches_at_idx = self
            .sections
            .get(section_idx)
            .and_then(|sec| sec.checks.get(check_idx))
            .is_some_and(|ch| crate::history::check_history_key(&ch.config) == check_key);

        if matches_at_idx {
            self.apply_result(section_idx, check_idx, res);
            return;
        }

        for (si, sec) in self.sections.iter().enumerate() {
            for (ci, ch) in sec.checks.iter().enumerate() {
                if crate::history::check_history_key(&ch.config) == check_key {
                    self.apply_result(si, ci, res);
                    return;
                }
            }
        }
    }

    pub fn apply_result(&mut self, section_idx: usize, check_idx: usize, res: CheckResult) {
        let lang = self.config.language;
        if let Some(sec) = self.sections.get_mut(section_idx) {
            if let Some(ch) = sec.checks.get_mut(check_idx) {
                let prev_ok = ch.result.as_ref().map(|r| r.ok);
                self.uptime_history
                    .record(&ch.config, res.timestamp.timestamp(), res.ok);
                ch.total_count = ch.total_count.saturating_add(1);
                if res.ok {
                    ch.ok_count = ch.ok_count.saturating_add(1);
                    ch.last_ok_at = Some(res.timestamp);
                }
                if ch.config.notify {
                    if let Some(notif) = build_status_notification(&ch.config, prev_ok, &res, lang) {
                        if self.pending_notifications.len() >= 32 {
                            self.pending_notifications.pop_front();
                        }
                        self.pending_notifications.push_back(notif);
                    }
                }
                ch.result = Some(res);
            }
        }
    }

    pub fn save_settings_to_disk(&mut self) {
        self.config.opacity = self.opacity;
        crate::config::save_config(&self.config);
        self.config_modified = std::fs::metadata(crate::config::config_path())
            .and_then(|m| m.modified())
            .ok();
    }

    pub fn sync_config_to_disk(&mut self) {
        self.config.opacity = self.opacity;
        crate::config::save_config_with_sections(&self.config);
        self.config_modified = std::fs::metadata(crate::config::config_path())
            .and_then(|m| m.modified())
            .ok();
        self.scheduler_needs_sync = true;
    }

    pub fn open_settings(&mut self) {
        if !self.show_settings {
            let size =
                crate::ui::settings::clamped_window_size(self.config.language, self.main_outer_rect);
            self.settings_open_pos = crate::monitor::place_child_near_main(
                self.main_outer_rect,
                size,
                self.config.settings_window_pos,
            );
            self.show_settings = true;
        }
    }

    pub fn open_edit_host(&mut self, edit: EditHostState) {
        if self.editing_check.is_none() && self.editing_group.is_none() {
            self.edit_open_pos = crate::monitor::place_child_near_main(
                self.main_outer_rect,
                eframe::egui::Vec2::new(360.0, 435.0),
                self.config.edit_window_pos,
            );
        }
        self.editing_group = None;
        self.editing_check = Some(edit);
    }

    pub fn open_edit_group(&mut self, group: EditGroupState) {
        if self.editing_check.is_none() && self.editing_group.is_none() {
            self.edit_open_pos = crate::monitor::place_child_near_main(
                self.main_outer_rect,
                eframe::egui::Vec2::new(340.0, 155.0),
                self.config.edit_window_pos,
            );
        }
        self.editing_check = None;
        self.editing_group = Some(group);
    }

    pub fn delete_check(&mut self, sec_idx: usize, chk_idx: usize) {
        if sec_idx < self.config.sections.len() && chk_idx < self.config.sections[sec_idx].checks.len() {
            self.config.sections[sec_idx].checks.remove(chk_idx);
        }
        if sec_idx < self.sections.len() && chk_idx < self.sections[sec_idx].checks.len() {
            self.sections[sec_idx].checks.remove(chk_idx);
        }
        self.sync_config_to_disk();
    }

    pub fn delete_section(&mut self, sec_idx: usize) {
        if sec_idx < self.config.sections.len() {
            self.config.sections.remove(sec_idx);
        }
        if sec_idx < self.sections.len() {
            self.sections.remove(sec_idx);
        }
        self.sync_config_to_disk();
    }

    pub fn move_check(&mut self, from: (usize, usize), to: (usize, usize)) -> (usize, usize) {
        let (fs, fc) = from;
        let (ts, mut tc) = to;
        if fs >= self.sections.len()
            || ts >= self.sections.len()
            || fc >= self.sections[fs].checks.len()
            || fs >= self.config.sections.len()
            || ts >= self.config.sections.len()
            || fc >= self.config.sections[fs].checks.len()
        {
            return from;
        }
        let cfg_item = self.config.sections[fs].checks.remove(fc);
        let st_item = self.sections[fs].checks.remove(fc);

        let dest_len = self.sections[ts].checks.len();
        if fs == ts && tc > dest_len {
            tc = dest_len;
        } else {
            tc = tc.min(dest_len);
        }
        self.config.sections[ts].checks.insert(tc, cfg_item);
        self.sections[ts].checks.insert(tc, st_item);
        (ts, tc)
    }

    pub fn swap_sections(&mut self, a: usize, b: usize) {
        if a < self.sections.len()
            && b < self.sections.len()
            && a < self.config.sections.len()
            && b < self.config.sections.len()
            && a != b
        {
            self.config.sections.swap(a, b);
            self.sections.swap(a, b);
        }
    }

    #[allow(dead_code)]
    pub fn error_count(&self) -> usize {
        let mut count = 0;
        for sec in &self.sections {
            for ch in &sec.checks {
                if !ch.config.is_optional() {
                    if let Some(r) = &ch.result {
                        if !r.ok {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }
}

/// Сообщение от планировщика к UI.
#[derive(Debug)]
pub enum CheckUpdate {
    Started,
    Result {
        section_idx: usize,
        check_idx: usize,
        check_key: String,
        result: CheckResult,
    },
    Finished,
}

/// Агрегированное состояние всех узлов для иконки трея.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverallStatus {
    Ok,
    Pending,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSummary {
    pub overall: OverallStatus,
    pub online_count: usize,
    pub total_count: usize,
    pub optional_down_count: usize,
    pub down_critical_names: Vec<String>,
}

impl AppState {
    pub fn overall_status(&self) -> OverallStatus {
        let mut any_err = false;
        let mut any_pending = false;
        for sec in &self.sections {
            for ch in &sec.checks {
                let opt = ch.config.is_optional();
                match &ch.result {
                    Some(r) if r.ok => {}
                    Some(_) => {
                        if !opt {
                            any_err = true;
                        }
                    }
                    None => {
                        if !opt {
                            any_pending = true;
                        }
                    }
                }
            }
        }
        if any_err {
            OverallStatus::Error
        } else if any_pending {
            OverallStatus::Pending
        } else {
            OverallStatus::Ok
        }
    }

    pub fn status_summary(&self) -> StatusSummary {
        let mut online_count = 0usize;
        let mut total_count = 0usize;
        let mut optional_down_count = 0usize;
        let mut down_critical_names = Vec::new();

        for sec in &self.sections {
            for ch in &sec.checks {
                total_count += 1;
                let opt = ch.config.is_optional();
                if let Some(r) = &ch.result {
                    if r.ok {
                        online_count += 1;
                    } else if opt {
                        optional_down_count += 1;
                    } else {
                        down_critical_names.push(ch.config.name.clone());
                    }
                }
            }
        }

        StatusSummary {
            overall: self.overall_status(),
            online_count,
            total_count,
            optional_down_count,
            down_critical_names,
        }
    }
}

fn build_status_notification(
    cfg: &CheckConfig,
    prev_ok: Option<bool>,
    res: &CheckResult,
    lang: crate::i18n::Language,
) -> Option<TrayNotification> {
    let s = crate::i18n::tr(lang);
    if !res.ok && prev_ok != Some(false) {
        let title = if cfg.is_optional() {
            s.notify_title_opt_down
        } else {
            s.notify_title_down
        };
        let reason = res
            .error
            .as_deref()
            .filter(|e| !e.is_empty())
            .map(|e| format!(" — {}", e))
            .unwrap_or_default();
        let body = format!("{} ({}) {}{}", cfg.name, cfg.host, s.notify_unreachable, reason);
        let level = if cfg.is_optional() {
            TrayNotifyLevel::Warning
        } else {
            TrayNotifyLevel::Error
        };
        return Some(TrayNotification {
            title: title.to_string(),
            body,
            level,
        });
    }
    if res.ok && prev_ok == Some(false) {
        let latency = res
            .latency_ms
            .map(|ms| format!(" ({} {})", ms, s.tt_ms))
            .unwrap_or_default();
        let body = format!(
            "{} ({}) {}{}",
            cfg.name, cfg.host, s.notify_back_online, latency
        );
        return Some(TrayNotification {
            title: s.notify_title_up.to_string(),
            body,
            level: TrayNotifyLevel::Info,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, CheckConfig, SectionConfig};

    fn make_check(name: &str, kind: &str, optional: bool) -> CheckConfig {
        CheckConfig {
            name: name.into(),
            host: "127.0.0.1".into(),
            kind: kind.into(),
            port: None,
            timeout_ms: 1000,
            interval_sec: None,
            optional,
            notify: false,
        }
    }

    fn make_res(ok: bool) -> CheckResult {
        CheckResult {
            ok,
            latency_ms: if ok { Some(2) } else { None },
            error: if ok { None } else { Some("Timeout".into()) },
            timestamp: Local::now(),
        }
    }

    #[test]
    fn optional_check_offline_keeps_overall_ok() {
        let cfg = AppConfig {
            sections: vec![SectionConfig {
                name: "LAN".into(),
                checks: vec![
                    make_check("Router", "ping", false),
                    make_check("Canon MFD", "ping_optional", false),
                ],
            }],
            ..Default::default()
        };
        let mut st = AppState::from_config(&cfg);
        st.apply_result(0, 0, make_res(true));
        st.apply_result(0, 1, make_res(false));

        let summary = st.status_summary();
        assert_eq!(summary.overall, OverallStatus::Ok);
        assert_eq!(summary.online_count, 1);
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.optional_down_count, 1);
        assert!(summary.down_critical_names.is_empty());
        assert_eq!(st.error_count(), 0);
    }

    #[test]
    fn critical_check_offline_triggers_error() {
        let cfg = AppConfig {
            sections: vec![SectionConfig {
                name: "Internet".into(),
                checks: vec![
                    make_check("DNS", "ping", false),
                    make_check("Canon MFD", "ping", true),
                ],
            }],
            ..Default::default()
        };
        let mut st = AppState::from_config(&cfg);
        st.apply_result(0, 0, make_res(false));
        st.apply_result(0, 1, make_res(false));

        let summary = st.status_summary();
        assert_eq!(summary.overall, OverallStatus::Error);
        assert_eq!(summary.down_critical_names, vec!["DNS".to_string()]);
        assert_eq!(summary.optional_down_count, 1);
        assert_eq!(st.error_count(), 1);
    }

    #[test]
    fn optional_port_defaults_and_toml_roundtrip() {
        let toml_src = r#"
[[sections]]
name = "Test"

[[sections.checks]]
name = "Router"
host = "192.168.1.1"
type = "ping"

[[sections.checks]]
name = "Legacy Zero Port"
host = "ya.ru"
type = "https"
port = 0

[[sections.checks]]
name = "Custom HTTP"
host = "192.168.1.10"
type = "http"
port = 8080
"#;
        let parsed: AppConfig = toml::from_str(toml_src).unwrap();
        let checks = &parsed.sections[0].checks;
        assert_eq!(checks[0].port, None);
        assert_eq!(checks[0].effective_port(), 0);
        assert_eq!(checks[1].port, None);
        assert_eq!(checks[1].effective_port(), 443);
        assert_eq!(checks[2].port, Some(8080));
        assert_eq!(checks[2].effective_port(), 8080);

        let serialized = toml::to_string_pretty(&parsed).unwrap();
        assert!(!serialized.contains("port = 0"));
        assert!(serialized.contains("port = 8080"));
    }

    #[test]
    fn move_checks_and_swap_sections_keep_config_and_state_in_sync() {
        let cfg = AppConfig {
            sections: vec![
                SectionConfig {
                    name: "G1".into(),
                    checks: vec![
                        make_check("A", "ping", false),
                        make_check("B", "ping", false),
                    ],
                },
                SectionConfig {
                    name: "G2".into(),
                    checks: vec![make_check("C", "ping", false)],
                },
            ],
            ..Default::default()
        };
        let mut st = AppState::from_config(&cfg);
        // Перемещение внутри группы
        assert_eq!(st.move_check((0, 0), (0, 1)), (0, 1));
        assert_eq!(st.sections[0].checks[0].config.name, "B");
        assert_eq!(st.config.sections[0].checks[1].name, "A");

        // Перемещение в другую группу
        assert_eq!(st.move_check((0, 1), (1, 0)), (1, 0));
        assert_eq!(st.sections[0].checks.len(), 1);
        assert_eq!(st.sections[1].checks[0].config.name, "A");
        assert_eq!(st.config.sections[1].checks[0].name, "A");

        // Перестановка групп
        st.swap_sections(0, 1);
        assert_eq!(st.sections[0].name, "G2");
        assert_eq!(st.config.sections[0].name, "G2");
        assert_eq!(st.sections[1].name, "G1");
    }

    #[test]
    fn custom_host_interval_and_theme_toml() {
        let toml_src = r#"
interval_sec = 10
theme = "light"

[[sections]]
name = "Test"

[[sections.checks]]
name = "Slow"
host = "1.1.1.1"
type = "dns"
interval_sec = 45

[[sections.checks]]
name = "Default"
host = "8.8.8.8"
type = "ping"
"#;
        let parsed: AppConfig = toml::from_str(toml_src).unwrap();
        assert_eq!(parsed.theme, crate::i18n::ThemeMode::Light);
        let c0 = &parsed.sections[0].checks[0];
        let c1 = &parsed.sections[0].checks[1];
        assert_eq!(c0.interval_sec, Some(45));
        assert_eq!(c0.effective_interval_sec(parsed.interval_sec), 45);
        assert_eq!(c1.interval_sec, None);
        assert_eq!(c1.effective_interval_sec(parsed.interval_sec), 10);
    }

    #[test]
    fn notifications_fire_only_on_transitions_when_enabled() {
        let mut silent = make_check("Silent", "ping", false);
        silent.notify = false;
        let mut loud = make_check("Router", "ping", false);
        loud.notify = true;

        let cfg = AppConfig {
            language: crate::i18n::Language::En,
            sections: vec![SectionConfig {
                name: "LAN".into(),
                checks: vec![silent, loud],
            }],
            ..Default::default()
        };
        let mut st = AppState::from_config(&cfg);

        // Silent host stays quiet even on offline/recovery
        st.apply_result(0, 0, make_res(false));
        st.apply_result(0, 0, make_res(true));
        assert!(st.pending_notifications.is_empty());

        // Loud host: Unknown -> Online is silent
        st.apply_result(0, 1, make_res(true));
        assert!(st.pending_notifications.is_empty());

        // Online -> Offline emits Error
        st.apply_result(0, 1, make_res(false));
        assert_eq!(st.pending_notifications.len(), 1);
        let n1 = st.pending_notifications.pop_front().unwrap();
        assert_eq!(n1.level, TrayNotifyLevel::Error);

        // Repeated Offline does not duplicate
        st.apply_result(0, 1, make_res(false));
        assert!(st.pending_notifications.is_empty());

        // Offline -> Online emits Info
        st.apply_result(0, 1, make_res(true));
        assert_eq!(st.pending_notifications.len(), 1);
        let n2 = st.pending_notifications.pop_front().unwrap();
        assert_eq!(n2.level, TrayNotifyLevel::Info);
    }

    #[test]
    fn reload_ignores_corrupted_toml_and_preserves_state() {
        let mut path = std::env::temp_dir();
        let unique = format!(
            "netmon_widget_reload_test_{}_{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        path.push(unique);

        let initial_cfg = AppConfig {
            interval_sec: 15,
            sections: vec![SectionConfig {
                name: "Core".into(),
                checks: vec![make_check("GW", "ping", false)],
            }],
            ..Default::default()
        };
        std::fs::write(&path, toml::to_string_pretty(&initial_cfg).unwrap()).unwrap();

        let mut st = AppState::from_config(&initial_cfg);
        st.config_modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        assert!(!st.reload_from_path_if_modified(&path));
        assert!(st.config_modified.is_some());

        // Overwrite with invalid TOML and advance mtime so hot-reload triggers.
        std::fs::write(&path, "interval_sec = [broken toml").unwrap();
        if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) {
            let _ = file.set_modified(
                std::time::SystemTime::now() + std::time::Duration::from_secs(5),
            );
        }

        let reloaded = st.reload_from_path_if_modified(&path);
        let _ = std::fs::remove_file(&path);
        let mut bak = path.clone();
        bak.set_extension("toml.bak");
        let _ = std::fs::remove_file(&bak);

        assert!(!reloaded);
        assert_eq!(st.config.interval_sec, 15);
        assert_eq!(st.sections.len(), 1);
        assert_eq!(st.sections[0].name, "Core");
        assert_eq!(st.sections[0].checks.len(), 1);
        assert_eq!(st.sections[0].checks[0].config.name, "GW");
    }
}
