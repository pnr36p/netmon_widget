use crate::checks;
use crate::config::AppConfig;
use crate::history::check_history_key;
use crate::state::CheckUpdate;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::watch;

fn wake_ui(egui_ctx: &Arc<Mutex<Option<eframe::egui::Context>>>) {
    if let Some(ctx) = egui_ctx.lock().unwrap().as_ref() {
        ctx.request_repaint();
    }
}

pub fn spawn_scheduler(
    mut cfg_rx: watch::Receiver<AppConfig>,
    tx: UnboundedSender<CheckUpdate>,
    mut trigger_rx: UnboundedReceiver<()>,
    egui_ctx: Arc<Mutex<Option<eframe::egui::Context>>>,
) {
    tokio::spawn(async move {
        let mut last_run: HashMap<(usize, usize, String), Instant> = HashMap::new();
        let mut force_all = true;

        loop {
            let current_cfg = cfg_rx.borrow().clone();
            let now = Instant::now();

            if !force_all {
                let mut next_due: Option<Instant> = None;
                for (si, section) in current_cfg.sections.iter().enumerate() {
                    for (ci, check) in section.checks.iter().enumerate() {
                        let key = (si, ci, check_history_key(check));
                        let eff = Duration::from_secs(
                            check.effective_interval_sec(current_cfg.interval_sec),
                        );
                        let due = match last_run.get(&key) {
                            Some(&prev) => prev + eff,
                            None => now,
                        };
                        next_due = Some(match next_due {
                            Some(cur) => cur.min(due),
                            None => due,
                        });
                    }
                }

                let target_instant =
                    next_due.unwrap_or_else(|| now + Duration::from_secs(current_cfg.interval_sec.max(1)));

                if target_instant > now {
                    tokio::select! {
                        _ = tokio::time::sleep_until(tokio::time::Instant::from_std(target_instant)) => {}
                        Some(_) = trigger_rx.recv() => {
                            force_all = true;
                        }
                        res = cfg_rx.changed() => {
                            if res.is_err() {
                                break;
                            }
                            continue;
                        }
                    }
                }
            }

            let run_all = force_all;
            force_all = false;
            while trigger_rx.try_recv().is_ok() {}

            let active_cfg = cfg_rx.borrow().clone();
            last_run.retain(|&(si, ci, ref k), _| {
                active_cfg
                    .sections
                    .get(si)
                    .and_then(|s| s.checks.get(ci))
                    .is_some_and(|c| &check_history_key(c) == k)
            });

            let check_now = Instant::now();
            let mut due_items = Vec::new();

            for (si, section) in active_cfg.sections.iter().enumerate() {
                for (ci, check) in section.checks.iter().enumerate() {
                    let key_str = check_history_key(check);
                    let key = (si, ci, key_str.clone());
                    let eff = Duration::from_secs(
                        check.effective_interval_sec(active_cfg.interval_sec),
                    );
                    let is_due = run_all
                        || match last_run.get(&key) {
                            Some(&prev) => check_now + Duration::from_millis(50) >= prev + eff,
                            None => true,
                        };
                    if is_due {
                        last_run.insert(key, check_now);
                        due_items.push((si, ci, key_str, check.clone()));
                    }
                }
            }

            if due_items.is_empty() {
                continue;
            }

            let _ = tx.send(CheckUpdate::Started);
            wake_ui(&egui_ctx);

            let mut handles = Vec::with_capacity(due_items.len());
            for (si, ci, check_key, check) in due_items {
                let tx = tx.clone();
                let egui_ctx = egui_ctx.clone();
                let handle = tokio::spawn(async move {
                    let res = checks::run_check(&check).await;
                    let _ = tx.send(CheckUpdate::Result {
                        section_idx: si,
                        check_idx: ci,
                        check_key,
                        result: res,
                    });
                    wake_ui(&egui_ctx);
                });
                handles.push(handle);
            }
            for h in handles {
                let _ = h.await;
            }

            let _ = tx.send(CheckUpdate::Finished);
            wake_ui(&egui_ctx);
        }
    });
}
