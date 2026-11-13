use crate::config::AppConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const WEEKLY_CHECK_SECS: i64 = 7 * 24 * 60 * 60;
const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/pnr36p/netmon_widget/releases/latest";

fn send_event(
    evt_tx: &mpsc::UnboundedSender<UpdateEvent>,
    egui_ctx: &Arc<Mutex<Option<eframe::egui::Context>>>,
    evt: UpdateEvent,
) {
    let _ = evt_tx.send(evt);
    if let Some(ctx) = egui_ctx.lock().unwrap().as_ref() {
        ctx.request_repaint();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Downloading(String),
    UpToDate,
    ReadyToRestart(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum UpdateCommand {
    CheckNow { manual: bool },
}

#[derive(Debug, Clone)]
pub enum UpdateEvent {
    CheckedAt(i64),
    Status(UpdateStatus),
    NotifyReady(String),
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

pub fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let clean = version
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()?;
    let mut parts = clean.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(remote_tag: &str, current: &str) -> bool {
    match (parse_semver(remote_tag), parse_semver(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

pub fn staged_exe_path(exe: &Path) -> PathBuf {
    let mut os = exe.as_os_str().to_os_string();
    os.push(".staged");
    PathBuf::from(os)
}

pub fn old_exe_path(exe: &Path) -> PathBuf {
    let mut os = exe.as_os_str().to_os_string();
    os.push(".old");
    PathBuf::from(os)
}

pub fn download_tmp_path(exe: &Path) -> PathBuf {
    let mut os = exe.as_os_str().to_os_string();
    os.push(".download");
    PathBuf::from(os)
}

pub fn is_valid_pe_header(header: &[u8], total_len: u64) -> bool {
    if total_len < 65_536 || header.len() < 64 || &header[0..2] != b"MZ" {
        return false;
    }
    let e_lfanew =
        u32::from_le_bytes([header[0x3C], header[0x3D], header[0x3E], header[0x3F]]) as usize;
    e_lfanew >= 64
        && e_lfanew.saturating_add(4) <= header.len()
        && &header[e_lfanew..e_lfanew + 4] == b"PE\0\0"
}

fn is_valid_pe_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let total_len = meta.len();
    if total_len < 65_536 {
        return false;
    }
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    use std::io::Read;
    let mut buf = [0u8; 4096];
    let mut read_len = 0usize;
    while read_len < buf.len() {
        match f.read(&mut buf[read_len..]) {
            Ok(0) => break,
            Ok(n) => read_len += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return false,
        }
    }
    is_valid_pe_header(&buf[..read_len], total_len)
}

/// Удаляет остаточные .old / .download файлы после предыдущего обновления.
pub fn cleanup_old_files() {
    let exe = crate::platform::current_exe_path();
    let _ = std::fs::remove_file(old_exe_path(&exe));
    let _ = std::fs::remove_file(download_tmp_path(&exe));
}

/// Применяет уже скачанное обновление (.staged) и перезапускает текущий .exe.
/// Возвращает Ok(true), если замена и запуск нового процесса выполнены успешно.
pub fn apply_staged_and_restart() -> Result<bool, String> {
    let exe = crate::platform::current_exe_path();
    let staged = staged_exe_path(&exe);
    if !staged.exists() {
        return Ok(false);
    }
    if !is_valid_pe_file(&staged) {
        let _ = std::fs::remove_file(&staged);
        return Err("Invalid staged update binary".to_string());
    }

    let old = old_exe_path(&exe);
    let _ = std::fs::remove_file(&old);

    if let Err(e) = std::fs::rename(&exe, &old) {
        return Err(format!("Rename current exe failed: {}", e));
    }
    if let Err(e) = std::fs::rename(&staged, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("Activate staged exe failed: {}", e));
    }

    match std::process::Command::new(&exe).spawn() {
        Ok(_) => Ok(true),
        Err(e) => {
            let _ = std::fs::rename(&exe, &staged);
            let _ = std::fs::rename(&old, &exe);
            Err(format!("Relaunch updated exe failed: {}", e))
        }
    }
}

pub fn spawn_updater(
    mut cmd_rx: mpsc::UnboundedReceiver<UpdateCommand>,
    mut cfg_rx: watch::Receiver<AppConfig>,
    evt_tx: mpsc::UnboundedSender<UpdateEvent>,
    egui_ctx: Arc<Mutex<Option<eframe::egui::Context>>>,
) {
    tokio::spawn(async move {
        // Если уже скачано обновление с прошлой сессии — сразу показываем готовность к перезапуску
        let exe = crate::platform::current_exe_path();
        if is_valid_pe_file(&staged_exe_path(&exe)) {
            send_event(
                &evt_tx,
                &egui_ctx,
                UpdateEvent::Status(UpdateStatus::ReadyToRestart("new".to_string())),
            );
        }

        // Небольшая пауза при старте, чтобы не мешать первому опросу узлов и отрисовке
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        {
            let cfg = cfg_rx.borrow().clone();
            let now = chrono::Utc::now().timestamp();
            if cfg.auto_update && (now - cfg.last_update_check_ts) >= WEEKLY_CHECK_SECS {
                run_check_and_download(&evt_tx, &egui_ctx, false).await;
            }
        }

        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Первый тик срабатывает мгновенно — пропустим его
        ticker.tick().await;

        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        UpdateCommand::CheckNow { manual } => {
                            run_check_and_download(&evt_tx, &egui_ctx, manual).await;
                        }
                    }
                }
                _ = ticker.tick() => {
                    let cfg = cfg_rx.borrow().clone();
                    let now = chrono::Utc::now().timestamp();
                    if cfg.auto_update && (now - cfg.last_update_check_ts) >= WEEKLY_CHECK_SECS {
                        run_check_and_download(&evt_tx, &egui_ctx, false).await;
                    }
                }
                res = cfg_rx.changed() => {
                    if res.is_err() {
                        break;
                    }
                }
            }
        }
    });
}

async fn run_check_and_download(
    evt_tx: &mpsc::UnboundedSender<UpdateEvent>,
    egui_ctx: &Arc<Mutex<Option<eframe::egui::Context>>>,
    manual: bool,
) {
    let report_err = |msg: String| {
        send_event(
            evt_tx,
            egui_ctx,
            UpdateEvent::Status(if manual {
                UpdateStatus::Error(msg)
            } else {
                UpdateStatus::Idle
            }),
        );
    };

    send_event(evt_tx, egui_ctx, UpdateEvent::Status(UpdateStatus::Checking));

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(format!("netmon_widget/{}", CURRENT_VERSION))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            report_err(e.to_string());
            return;
        }
    };

    let release: GitHubRelease = match client.get(GITHUB_LATEST_RELEASE_URL).send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(ok) => match ok.json::<GitHubRelease>().await {
                Ok(rel) => rel,
                Err(e) => {
                    report_err(e.to_string());
                    return;
                }
            },
            Err(e) => {
                report_err(e.to_string());
                return;
            }
        },
        Err(e) => {
            report_err(e.to_string());
            return;
        }
    };

    send_event(
        evt_tx,
        egui_ctx,
        UpdateEvent::CheckedAt(chrono::Utc::now().timestamp()),
    );

    if !is_newer_version(&release.tag_name, CURRENT_VERSION) {
        send_event(evt_tx, egui_ctx, UpdateEvent::Status(UpdateStatus::UpToDate));
        return;
    }

    let remote_ver = release
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();

    let Some(asset) = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("netmon_widget.exe"))
    else {
        report_err("Release asset netmon_widget.exe not found".to_string());
        return;
    };

    let exe = crate::platform::current_exe_path();
    let staged = staged_exe_path(&exe);
    if asset.size > 0
        && std::fs::metadata(&staged).map(|m| m.len()).unwrap_or(0) == asset.size
        && is_valid_pe_file(&staged)
    {
        send_event(
            evt_tx,
            egui_ctx,
            UpdateEvent::Status(UpdateStatus::ReadyToRestart(remote_ver)),
        );
        return;
    }

    send_event(
        evt_tx,
        egui_ctx,
        UpdateEvent::Status(UpdateStatus::Downloading(remote_ver.clone())),
    );

    let dl_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .user_agent(format!("netmon_widget/{}", CURRENT_VERSION))
        .build()
        .unwrap_or(client);

    let bytes = match dl_client.get(&asset.browser_download_url).send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(ok) => match ok.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    report_err(e.to_string());
                    return;
                }
            },
            Err(e) => {
                report_err(e.to_string());
                return;
            }
        },
        Err(e) => {
            report_err(e.to_string());
            return;
        }
    };

    if asset.size > 0 && bytes.len() as u64 != asset.size {
        report_err(format!(
            "Downloaded size mismatch: expected {} bytes, got {}",
            asset.size,
            bytes.len()
        ));
        return;
    }

    if !is_valid_pe_header(&bytes, bytes.len() as u64) {
        report_err("Downloaded file is not a valid Windows executable".to_string());
        return;
    }

    let tmp = download_tmp_path(&exe);
    if let Err(e) = std::fs::write(&tmp, &bytes) {
        report_err(e.to_string());
        return;
    }
    let _ = std::fs::remove_file(&staged);
    if let Err(e) = std::fs::rename(&tmp, &staged) {
        let _ = std::fs::remove_file(&tmp);
        report_err(e.to_string());
        return;
    }

    send_event(
        evt_tx,
        egui_ctx,
        UpdateEvent::Status(UpdateStatus::ReadyToRestart(remote_ver.clone())),
    );
    send_event(evt_tx, egui_ctx, UpdateEvent::NotifyReady(remote_ver));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_handles_v_prefix_and_prerelease() {
        assert!(is_newer_version("v0.1.1", "0.1.0"));
        assert!(is_newer_version("v0.2.0", "0.1.9"));
        assert!(is_newer_version("v1.0.0", "0.9.9"));
        assert!(!is_newer_version("v0.1.0", "0.1.0"));
        assert!(!is_newer_version("v0.0.9", "0.1.0"));
        assert_eq!(parse_semver("V1.2.3-beta1"), Some((1, 2, 3)));
    }

    #[test]
    fn validates_pe_header_signature_and_size() {
        let mut valid = vec![0u8; 256];
        valid[0..2].copy_from_slice(b"MZ");
        valid[0x3C..0x40].copy_from_slice(&(128u32).to_le_bytes());
        valid[128..132].copy_from_slice(b"PE\0\0");

        assert!(is_valid_pe_header(&valid, 70_000));
        assert!(!is_valid_pe_header(&valid, 10_000));

        let mut missing_pe = valid.clone();
        missing_pe[128..132].copy_from_slice(b"NOPE");
        assert!(!is_valid_pe_header(&missing_pe, 70_000));

        let mut bad_mz = valid.clone();
        bad_mz[0..2].copy_from_slice(b"PK");
        assert!(!is_valid_pe_header(&bad_mz, 70_000));

        let mut bad_offset = valid;
        bad_offset[0x3C..0x40].copy_from_slice(&(16u32).to_le_bytes());
        assert!(!is_valid_pe_header(&bad_offset, 70_000));
    }
}
