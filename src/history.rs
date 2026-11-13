use crate::config::{config_path, CheckConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// Размер одного агрегационного бакета: 30 минут (1800 секунд).
/// Дает хорошую точность суточного окна и компактный файл истории (~до 500 КБ за 30 дней).
const BUCKET_SECS: i64 = 1800;
const DAY_SECS: i64 = 86_400;
const WEEK_SECS: i64 = 7 * DAY_SECS;
const MONTH_SECS: i64 = 30 * DAY_SECS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UptimeBucket {
    pub bucket_ts: i64,
    pub ok: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowUptime {
    pub ok: u64,
    pub total: u64,
}

impl WindowUptime {
    pub fn percent(&self) -> Option<f32> {
        if self.total == 0 {
            None
        } else {
            Some(self.ok as f32 / self.total as f32 * 100.0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UptimeWindows {
    pub day: WindowUptime,
    pub week: WindowUptime,
    pub month: WindowUptime,
}

#[derive(Debug, Default)]
pub struct UptimeHistory {
    series: HashMap<String, Vec<UptimeBucket>>,
    dirty: bool,
}

pub fn check_history_key(cfg: &CheckConfig) -> String {
    format!("{}|{}|{}", cfg.base_kind(), cfg.host.trim(), cfg.effective_port())
}

pub fn history_path() -> PathBuf {
    let mut p = config_path();
    p.set_file_name("netmon_history.dat");
    p
}

impl UptimeHistory {
    pub fn load() -> Self {
        let path = history_path();
        let Ok(content) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        let now = chrono::Utc::now().timestamp();
        Self::parse_str(&content, now)
    }

    pub fn parse_str(content: &str, now_sec: i64) -> Self {
        let cutoff = (now_sec - MONTH_SECS).div_euclid(BUCKET_SECS) * BUCKET_SECS;
        let mut series: HashMap<String, Vec<UptimeBucket>> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, rest)) = line.split_once('\t') else {
                continue;
            };
            let mut buckets = Vec::new();
            for item in rest.split(';') {
                let mut parts = item.split(',');
                let (Some(ts_s), Some(ok_s), Some(tot_s)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };
                if let (Ok(ts), Ok(ok), Ok(total)) =
                    (ts_s.parse::<i64>(), ok_s.parse::<u32>(), tot_s.parse::<u32>())
                {
                    if ts >= cutoff && total > 0 {
                        buckets.push(UptimeBucket {
                            bucket_ts: ts,
                            ok: ok.min(total),
                            total,
                        });
                    }
                }
            }
            if !buckets.is_empty() {
                buckets.sort_by_key(|b| b.bucket_ts);
                series.insert(key.to_string(), buckets);
            }
        }

        Self {
            series,
            dirty: false,
        }
    }

    pub fn serialize_str(&self) -> String {
        let mut out = String::new();
        let mut keys: Vec<&String> = self.series.keys().collect();
        keys.sort();
        for k in keys {
            let Some(buckets) = self.series.get(k) else {
                continue;
            };
            if buckets.is_empty() {
                continue;
            }
            out.push_str(k);
            out.push('\t');
            for (i, b) in buckets.iter().enumerate() {
                if i > 0 {
                    out.push(';');
                }
                out.push_str(&format!("{},{},{}", b.bucket_ts, b.ok, b.total));
            }
            out.push('\n');
        }
        out
    }

    pub fn record(&mut self, cfg: &CheckConfig, timestamp_sec: i64, ok: bool) {
        let key = check_history_key(cfg);
        let bucket_ts = timestamp_sec.div_euclid(BUCKET_SECS) * BUCKET_SECS;
        let cutoff = (timestamp_sec - MONTH_SECS).div_euclid(BUCKET_SECS) * BUCKET_SECS;

        let list = self.series.entry(key).or_default();
        if let Some(last) = list.last_mut().filter(|b| b.bucket_ts == bucket_ts) {
            last.total = last.total.saturating_add(1);
            if ok {
                last.ok = last.ok.saturating_add(1);
            }
        } else if let Some(found) = list.iter_mut().find(|b| b.bucket_ts == bucket_ts) {
            found.total = found.total.saturating_add(1);
            if ok {
                found.ok = found.ok.saturating_add(1);
            }
        } else {
            list.push(UptimeBucket {
                bucket_ts,
                ok: if ok { 1 } else { 0 },
                total: 1,
            });
            list.sort_by_key(|b| b.bucket_ts);
        }

        if list.first().is_some_and(|b| b.bucket_ts < cutoff) {
            list.retain(|b| b.bucket_ts >= cutoff);
        }
        self.dirty = true;
    }

    pub fn windows(&self, cfg: &CheckConfig, now_sec: i64) -> UptimeWindows {
        let key = check_history_key(cfg);
        let Some(buckets) = self.series.get(&key) else {
            return UptimeWindows {
                day: WindowUptime { ok: 0, total: 0 },
                week: WindowUptime { ok: 0, total: 0 },
                month: WindowUptime { ok: 0, total: 0 },
            };
        };

        let cut_day = (now_sec - DAY_SECS).div_euclid(BUCKET_SECS) * BUCKET_SECS;
        let cut_week = (now_sec - WEEK_SECS).div_euclid(BUCKET_SECS) * BUCKET_SECS;
        let cut_month = (now_sec - MONTH_SECS).div_euclid(BUCKET_SECS) * BUCKET_SECS;

        let mut day = WindowUptime { ok: 0, total: 0 };
        let mut week = WindowUptime { ok: 0, total: 0 };
        let mut month = WindowUptime { ok: 0, total: 0 };

        for b in buckets {
            if b.bucket_ts >= cut_month {
                month.ok += b.ok as u64;
                month.total += b.total as u64;
            }
            if b.bucket_ts >= cut_week {
                week.ok += b.ok as u64;
                week.total += b.total as u64;
            }
            if b.bucket_ts >= cut_day {
                day.ok += b.ok as u64;
                day.total += b.total as u64;
            }
        }

        UptimeWindows { day, week, month }
    }

    pub fn save_if_dirty(&mut self) {
        if !self.dirty {
            return;
        }
        let content = self.serialize_str();
        if crate::config::atomic_write(&history_path(), content.as_bytes()).is_ok() {
            self.dirty = false;
        }
    }

    pub fn force_save(&mut self) {
        let content = self.serialize_str();
        if crate::config::atomic_write(&history_path(), content.as_bytes()).is_ok() {
            self.dirty = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cfg() -> CheckConfig {
        CheckConfig {
            name: "DNS".into(),
            host: "1.1.1.1".into(),
            kind: "dns".into(),
            port: None,
            timeout_ms: 2000,
            interval_sec: None,
            optional: false,
            notify: false,
        }
    }

    #[test]
    fn windows_aggregate_day_week_month_and_prune_old() {
        let cfg = sample_cfg();
        let now = 1_700_000_000i64;
        let mut hist = UptimeHistory::default();

        // 40 days ago (should be pruned)
        hist.record(&cfg, now - 40 * DAY_SECS, false);
        // 20 days ago (month only)
        hist.record(&cfg, now - 20 * DAY_SECS, false);
        // 3 days ago (week + month)
        hist.record(&cfg, now - 3 * DAY_SECS, true);
        // 2 hours ago (day + week + month)
        hist.record(&cfg, now - 7200, true);
        hist.record(&cfg, now - 7190, false);

        let w = hist.windows(&cfg, now);
        assert_eq!(w.day.ok, 1);
        assert_eq!(w.day.total, 2);
        assert_eq!(w.week.ok, 2);
        assert_eq!(w.week.total, 3);
        assert_eq!(w.month.ok, 2);
        assert_eq!(w.month.total, 4);

        let serialized = hist.serialize_str();
        let reloaded = UptimeHistory::parse_str(&serialized, now);
        assert_eq!(reloaded.windows(&cfg, now), w);
    }
}
