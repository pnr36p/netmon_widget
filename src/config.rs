use crate::i18n::{Language, ThemeMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CONFIG_HELP_HEADER_RU: &str = "\
# ============================================================================
# NetMon Widget — Конфигурация (изменения применяются автоматически)
# ============================================================================
# Доступные типы сенсоров (поле type):
#   ping          — ICMP Ping
#   tcp           — проверка подключения к TCP-порту (по умолчанию 80)
#   http / https  — HTTP/HTTPS GET (ожидает ответ 2xx/3xx; порты 80 / 443)
#   dns           — живой DNS-запрос (к DNS-серверу по IP:53 или резолв домена)
#
# Некритичные сенсоры (при отключении показывают жёлтый минус вместо красного
# крестика и НЕ переводят иконку в трее в состояние сбоя):
#   ping_optional, tcp_optional, http_optional, https_optional, dns_optional
#   (либо добавьте optional = true к любому обычному типу)
#
# Поля элемента [[sections.checks]]:
#   name         — отображаемое название
#   host         — IP-адрес или доменное имя
#   type         — тип сенсора (см. выше)
#   port         — [опционально] порт (для tcp/http/https/dns; для ping не нужен)
#   timeout_ms   — [опционально] таймаут в мс (по умолчанию 3000)
#   interval_sec — [опционально] собственный интервал опроса в сек (по умолч. общий)
#   optional     — [опционально] true = некритичный узел (по умолчанию false)
#   notify       — [опционально] true = всплывающие оповещения при сбое/восстановлении (по умолч. false)
# ============================================================================

";

pub const CONFIG_HELP_HEADER_EN: &str = "\
# ============================================================================
# NetMon Widget — Configuration (changes are reloaded automatically)
# ============================================================================
# Available sensor types (field `type`):
#   ping          — ICMP Ping
#   tcp           — TCP port connect check (default port 80)
#   http / https  — HTTP/HTTPS GET (expects 2xx/3xx; default ports 80 / 443)
#   dns           — live DNS query (UDP query to DNS server IP:53 or domain)
#
# Non-critical sensors (show yellow minus instead of red cross when offline
# and do NOT mark the system tray icon as error):
#   ping_optional, tcp_optional, http_optional, https_optional, dns_optional
#   (or add optional = true to any regular sensor type)
#
# Fields in [[sections.checks]]:
#   name         — display name
#   host         — IP address or domain name
#   type         — sensor type (see above)
#   port         — [optional] port (for tcp/http/https/dns; unused by ping)
#   timeout_ms   — [optional] timeout in ms (default 3000)
#   interval_sec — [optional] custom poll interval in sec (default: global)
#   optional     — [optional] true = non-critical host (default false)
#   notify       — [optional] true = popup notifications on status change (default false)
# ============================================================================

";

pub const CONFIG_HELP_HEADER_HR: &str = "\
# ============================================================================
# NetMon Widget — Konfiguracija (promjene se primjenjuju automatski)
# ============================================================================
# Dostupne vrste senzora (polje type):
#   ping          — ICMP Ping
#   tcp           — provjera povezivanja na TCP port (zadani port 80)
#   http / https  — HTTP/HTTPS GET (očekuje odgovor 2xx/3xx; zadani portovi 80 / 443)
#   dns           — UDP DNS upit prema DNS poslužitelju (IP:53) ili razrješavanje domene
#
# Nekritični senzori (pri nedostupnosti prikazuju žuti minus umjesto crvenog
# križića i NE označavaju ikonu u trayu kao grešku):
#   ping_optional, tcp_optional, http_optional, https_optional, dns_optional
#   (ili dodajte optional = true bilo kojem standardnom tipu)
#
# Polja stavke [[sections.checks]]:
#   name         — prikazani naziv
#   host         — IP adresa ili naziv domene
#   type         — vrsta senzora (vidi gore)
#   port         — [neobvezno] port (za tcp/http/https/dns; ping ga ne koristi)
#   timeout_ms   — [neobvezno] istek vremena u ms (zadano 3000)
#   interval_sec — [neobvezno] vlastiti interval provjere u sek (zadano: opći)
#   optional     — [neobvezno] true = nekritični čvor (zadano false)
#   notify       — [neobvezno] true = skočne obavijesti pri promjeni stanja (zadano false)
# ============================================================================

";

pub fn help_header(lang: Language) -> &'static str {
    match lang {
        Language::Ru => CONFIG_HELP_HEADER_RU,
        Language::En => CONFIG_HELP_HEADER_EN,
        Language::Hr => CONFIG_HELP_HEADER_HR,
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn deserialize_opt_port<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<u16> = serde::Deserialize::deserialize(deserializer)?;
    Ok(opt.filter(|&p| p > 0))
}

fn deserialize_opt_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<u32> = serde::Deserialize::deserialize(deserializer)?;
    Ok(opt.filter(|&v| v > 0))
}

fn deserialize_opt_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<u64> = serde::Deserialize::deserialize(deserializer)?;
    Ok(opt.filter(|&v| v > 0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConfig {
    pub name: String,
    pub host: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_port",
        skip_serializing_if = "Option::is_none"
    )]
    pub port: Option<u16>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub interval_sec: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub notify: bool,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u32",
        skip_serializing_if = "Option::is_none"
    )]
    pub fail_threshold: Option<u32>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub warn_ms: Option<u64>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_port",
        skip_serializing_if = "Option::is_none"
    )]
    pub expect_status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_text: Option<String>,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            kind: "ping".into(),
            port: None,
            timeout_ms: default_timeout(),
            interval_sec: None,
            optional: false,
            notify: false,
            fail_threshold: None,
            warn_ms: None,
            expect_status: None,
            expect_text: None,
        }
    }
}

impl CheckConfig {
    /// Некритичная проверка: при недоступности помечается жёлтым значком вместо красного креста
    /// и не переводит общую иконку трея в состояние ошибки.
    pub fn is_optional(&self) -> bool {
        self.optional
            || matches!(
                self.kind.as_str(),
                "ping_optional"
                    | "optional"
                    | "soft"
                    | "ping?"
                    | "tcp_optional"
                    | "http_optional"
                    | "https_optional"
                    | "dns_optional"
            )
    }

    /// Базовый протокол проверки для маршрутизации в `checks::run_check`.
    pub fn base_kind(&self) -> &str {
        match self.kind.as_str() {
            "ping_optional" | "optional" | "soft" | "ping?" => "ping",
            "tcp_optional" => "tcp",
            "http_optional" => "http",
            "https_optional" => "https",
            "dns_optional" => "dns",
            other => other,
        }
    }

    /// Эффективный порт с учетом протокола (когда `port` не задан в TOML).
    pub fn effective_port(&self) -> u16 {
        match self.port.filter(|&p| p > 0) {
            Some(p) => p,
            None => match self.base_kind() {
                "https" => 443,
                "http" | "tcp" => 80,
                "dns" => 53,
                _ => 0,
            },
        }
    }

    pub fn effective_interval_sec(&self, global_interval_sec: u64) -> u64 {
        self.interval_sec
            .filter(|&v| v > 0)
            .unwrap_or(global_interval_sec)
            .max(1)
    }

    pub fn effective_fail_threshold(&self) -> u32 {
        self.fail_threshold.unwrap_or(1).max(1)
    }
}

fn default_timeout() -> u64 {
    3000
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SectionConfig {
    pub name: String,
    #[serde(default)]
    pub checks: Vec<CheckConfig>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub collapsed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SizePreset {
    #[serde(rename = "xs")]
    Xs,
    #[serde(rename = "s")]
    #[default]
    S,
    #[serde(rename = "m")]
    M,
    #[serde(rename = "l")]
    L,
    #[serde(rename = "xl")]
    Xl,
}

impl SizePreset {
    pub fn width(&self) -> f32 {
        match self {
            SizePreset::Xs => 120.0,
            SizePreset::S => 130.0, // Стандартная ширина гаджета Windows Sidebar
            SizePreset::M => 150.0,
            SizePreset::L => 170.0,
            SizePreset::Xl => 200.0,
        }
    }

    pub fn font_size(&self) -> f32 {
        match self {
            SizePreset::Xs => 10.0,
            SizePreset::S => 11.0,
            SizePreset::M => 12.0,
            SizePreset::L => 13.0,
            SizePreset::Xl => 14.0,
        }
    }

    pub fn header_font_size(&self) -> f32 {
        match self {
            SizePreset::Xs => 9.5,
            SizePreset::S => 10.5,
            SizePreset::M => 11.5,
            SizePreset::L => 12.5,
            SizePreset::Xl => 13.5,
        }
    }

    pub fn row_height(&self) -> f32 {
        match self {
            SizePreset::Xs => 16.0,
            SizePreset::S => 17.5,
            SizePreset::M => 20.0,
            SizePreset::L => 22.0,
            SizePreset::Xl => 24.0,
        }
    }

    pub fn header_height(&self) -> f32 {
        12.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_interval")]
    pub interval_sec: u64,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default = "default_true")]
    pub edge_snapping: bool,
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default)]
    pub last_update_check_ts: i64,
    #[serde(default, alias = "tray_only")]
    pub hide_taskbar: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_latency_in_list: bool,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default)]
    pub size: SizePreset,
    #[serde(default)]
    pub custom_width: Option<f32>,
    #[serde(default)]
    pub custom_height: Option<f32>,
    #[serde(default)]
    pub window_pos: Option<[f32; 2]>,
    #[serde(default)]
    pub settings_window_pos: Option<[f32; 2]>,
    #[serde(default)]
    pub edit_window_pos: Option<[f32; 2]>,
    #[serde(default)]
    pub monitor_id: Option<String>,
    #[serde(default)]
    pub rel_pos: Option<[f32; 2]>,
    #[serde(default)]
    pub fallback_active: bool,
    #[serde(default)]
    pub sections: Vec<SectionConfig>,
}

fn default_interval() -> u64 {
    5
}
fn default_opacity() -> f32 {
    0.85
}
fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            interval_sec: 5,
            opacity: 0.85,
            always_on_top: true,
            edge_snapping: true,
            auto_update: true,
            last_update_check_ts: 0,
            hide_taskbar: false,
            show_latency_in_list: false,
            language: Language::Ru,
            theme: ThemeMode::System,
            size: SizePreset::S,
            custom_width: None,
            custom_height: None,
            window_pos: None,
            settings_window_pos: None,
            edit_window_pos: None,
            monitor_id: None,
            rel_pos: None,
            fallback_active: false,
            sections: vec![],
        }
    }
}

fn exe_dir() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p
}

fn is_cargo_target_dir(dir: &std::path::Path) -> bool {
    let s = dir.to_string_lossy().to_lowercase();
    s.ends_with("\\target\\debug")
        || s.ends_with("\\target\\release")
        || s.contains("\\target\\debug\\")
        || s.contains("\\target\\release\\")
}

/// Портативный путь к Network.csv: приоритет — папка exe (изоляция нескольких копий).
pub fn network_csv_path() -> PathBuf {
    let dir = exe_dir();
    let e1 = dir.join("Network.csv");
    if e1.exists() {
        return e1;
    }
    let e2 = dir.join("network.csv");
    if e2.exists() {
        return e2;
    }
    if is_cargo_target_dir(&dir) {
        let p1 = PathBuf::from("Network.csv");
        if p1.exists() {
            return p1;
        }
        let p2 = PathBuf::from("network.csv");
        if p2.exists() {
            return p2;
        }
    }
    e1
}

/// Декодирует байты в строку с поддержкой как UTF-8, так и Windows-1251 (CP1251)
pub fn decode_text_lossy_cp1251(bytes: &[u8]) -> String {
    if let Ok(utf8) = std::str::from_utf8(bytes) {
        return utf8.to_string();
    }
    bytes
        .iter()
        .map(|&b| match b {
            0x00..=0x7F => b as char,
            0xA8 => 'Ё',
            0xB8 => 'ё',
            0xAA => 'Є',
            0xBA => 'є',
            0xAF => 'Ї',
            0xBF => 'ї',
            0xB2 => 'І',
            0xB3 => 'і',
            0xA5 => 'Ґ',
            0xB4 => 'ґ',
            0xA0 => '\u{00A0}',
            0xC0..=0xDF => char::from_u32(0x0410 + (b - 0xC0) as u32).unwrap_or('?'),
            0xE0..=0xFF => char::from_u32(0x0430 + (b - 0xE0) as u32).unwrap_or('?'),
            _ => '?',
        })
        .collect()
}

/// Импортирует секции и хосты из формата Network.csv оригинального Windows 7 Gadget
pub fn import_network_csv(path: &std::path::Path) -> Result<Vec<SectionConfig>, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("Не удалось прочитать {}: {}", path.display(), e))?;
    let text = decode_text_lossy_cp1251(&bytes);

    let mut sections: Vec<SectionConfig> = Vec::new();
    let mut current_sec: Option<SectionConfig> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('<') {
            if let Some(sec) = current_sec.take() {
                sections.push(sec);
            }
            let name = line
                .trim_start_matches('<')
                .split('>')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            current_sec = Some(SectionConfig {
                name,
                checks: Vec::new(),
                collapsed: false,
            });
        } else if let Some(ref mut sec) = current_sec {
            if let Some((name_part, host_part)) = line.split_once(',') {
                let name = name_part.trim().to_string();
                let host = host_part.trim().trim_end_matches(',').trim().to_string();
                if !name.is_empty() && !host.is_empty() {
                    sec.checks.push(CheckConfig {
                        name,
                        host,
                        kind: "ping".into(),
                        timeout_ms: 2000,
                        ..Default::default()
                    });
                }
            }
        }
    }

    if let Some(sec) = current_sec {
        sections.push(sec);
    }

    if sections.is_empty() {
        return Err("В файле не найдено секций <ИмяСекции>".into());
    }

    Ok(sections)
}

/// Экспортирует секции и хосты в формат Network.csv.
pub fn export_network_csv(path: &std::path::Path, sections: &[SectionConfig]) -> Result<(), String> {
    let mut out = String::new();
    for (idx, sec) in sections.iter().enumerate() {
        if idx > 0 {
            out.push_str("\r\n");
        }
        out.push('<');
        out.push_str(sec.name.trim());
        out.push_str(">\r\n");
        for chk in &sec.checks {
            out.push_str(chk.name.trim());
            out.push_str(", ");
            out.push_str(chk.host.trim());
            out.push_str("\r\n");
        }
    }
    atomic_write(path, out.as_bytes())
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

/// Портативный путь к конфигу: всегда рядом с exe (чтобы две копии из разных папок не мешали друг другу).
/// При запуске из target/(debug|release) через cargo допускается чтение из рабочей директории.
pub fn config_path() -> PathBuf {
    let dir = exe_dir();
    let exe_cfg = dir.join("netmon_widget.toml");
    if exe_cfg.exists() {
        return exe_cfg;
    }
    if is_cargo_target_dir(&dir) {
        let cwd_path = PathBuf::from("netmon_widget.toml");
        if cwd_path.exists() {
            return cwd_path;
        }
    }
    exe_cfg
}

pub fn atomic_write(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    let mut tmp_os = path.as_os_str().to_os_string();
    tmp_os.push(".tmp");
    let tmp_path = PathBuf::from(tmp_os);
    std::fs::write(&tmp_path, data)?;
    if std::fs::rename(&tmp_path, path).is_err() {
        let res = std::fs::write(path, data);
        let _ = std::fs::remove_file(&tmp_path);
        return res;
    }
    Ok(())
}

fn backup_corrupted_config(path: &std::path::Path) {
    let mut bak_os = path.as_os_str().to_os_string();
    bak_os.push(".bak");
    let _ = std::fs::copy(path, PathBuf::from(bak_os));
}

fn write_toml_with_header(path: &std::path::Path, cfg: &AppConfig) {
    if let Ok(s) = toml::to_string_pretty(cfg) {
        let content = format!("{}{}", help_header(cfg.language), s);
        let _ = atomic_write(path, content.as_bytes());
    }
}

pub fn try_load_config_from(path: &std::path::Path) -> Result<AppConfig, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    toml::from_str::<AppConfig>(&text).map_err(|e| format!("TOML parse error: {}", e))
}

fn example_config_path() -> Option<PathBuf> {
    let dir = exe_dir();
    let exe_ex = dir.join("netmon_widget.example.toml");
    if exe_ex.exists() {
        return Some(exe_ex);
    }
    if is_cargo_target_dir(&dir) {
        let cwd_ex = PathBuf::from("netmon_widget.example.toml");
        if cwd_ex.exists() {
            return Some(cwd_ex);
        }
    }
    None
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    let mut cfg: AppConfig = if let Ok(text) = std::fs::read_to_string(&path) {
        match toml::from_str::<AppConfig>(&text) {
            Ok(c) => c,
            Err(_) => {
                backup_corrupted_config(&path);
                AppConfig::default()
            }
        }
    } else if let Some(ex_path) = example_config_path() {
        std::fs::read_to_string(&ex_path)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    } else {
        AppConfig::default()
    };

    // Если секции отсутствуют или пусты, проверяем наличие Network.csv
    if cfg.sections.is_empty() {
        let csv_p = network_csv_path();
        if csv_p.exists() {
            if let Ok(imported) = import_network_csv(&csv_p) {
                cfg.sections = imported;
            }
        }
    }

    // Если и Network.csv нет, создаем базовый пример
    if cfg.sections.is_empty() {
        cfg.sections = vec![SectionConfig {
            name: "Internet".into(),
            collapsed: false,
            checks: vec![
                CheckConfig {
                    name: "DNS Cloudflare".into(),
                    host: "1.1.1.1".into(),
                    kind: "ping".into(),
                    timeout_ms: 2000,
                    ..Default::default()
                },
                CheckConfig {
                    name: "Google DNS".into(),
                    host: "8.8.8.8".into(),
                    kind: "dns".into(),
                    timeout_ms: 2000,
                    ..Default::default()
                },
                CheckConfig {
                    name: "Google HTTPS".into(),
                    host: "www.google.com".into(),
                    kind: "https".into(),
                    port: Some(443),
                    timeout_ms: 3000,
                    ..Default::default()
                },
            ],
        }];
    }

    if !path.exists() {
        write_toml_with_header(&path, &cfg);
    }

    cfg
}

/// Сохраняет настройки гаджета, НЕ затирая вручную отредактированные хосты и секции.
pub fn save_config(cfg: &AppConfig) {
    let path = config_path();
    if let Ok(text) = std::fs::read_to_string(&path) {
        match toml::from_str::<AppConfig>(&text) {
            Ok(mut disk_cfg) => {
                disk_cfg.interval_sec = cfg.interval_sec;
                disk_cfg.opacity = cfg.opacity;
                disk_cfg.always_on_top = cfg.always_on_top;
                disk_cfg.edge_snapping = cfg.edge_snapping;
                disk_cfg.auto_update = cfg.auto_update;
                disk_cfg.last_update_check_ts = cfg.last_update_check_ts;
                disk_cfg.hide_taskbar = cfg.hide_taskbar;
                disk_cfg.show_latency_in_list = cfg.show_latency_in_list;
                disk_cfg.language = cfg.language;
                disk_cfg.theme = cfg.theme;
                disk_cfg.size = cfg.size;
                disk_cfg.custom_width = cfg.custom_width;
                disk_cfg.custom_height = cfg.custom_height;
                disk_cfg.window_pos = cfg.window_pos;
                disk_cfg.settings_window_pos = cfg.settings_window_pos;
                disk_cfg.edit_window_pos = cfg.edit_window_pos;
                disk_cfg.monitor_id = cfg.monitor_id.clone();
                disk_cfg.rel_pos = cfg.rel_pos;
                disk_cfg.fallback_active = cfg.fallback_active;
                if disk_cfg.sections.is_empty() && !cfg.sections.is_empty() {
                    disk_cfg.sections = cfg.sections.clone();
                }
                write_toml_with_header(&path, &disk_cfg);
                return;
            }
            Err(_) => {
                backup_corrupted_config(&path);
            }
        }
    }
    write_toml_with_header(&path, cfg);
}

/// Полное сохранение конфига (включая секции/хосты) при редактировании через UI или импорте CSV.
pub fn save_config_with_sections(cfg: &AppConfig) {
    let path = config_path();
    write_toml_with_header(&path, cfg);
}
