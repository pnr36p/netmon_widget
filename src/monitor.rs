#![allow(clippy::upper_case_acronyms)]

use eframe::egui::{Pos2, Rect, Vec2};
use std::ffi::c_void;

type HMONITOR = *mut c_void;
type HDC = *mut c_void;
type BOOL = i32;
type LPARAM = isize;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct MONITORINFOEXW {
    pub cb_size: u32,
    pub rc_monitor: RECT,
    pub rc_work: RECT,
    pub dw_flags: u32,
    pub sz_device: [u16; 32],
}

type MONITORENUMPROC = unsafe extern "system" fn(
    hmonitor: HMONITOR,
    hdc: HDC,
    lprect: *mut RECT,
    lparam: LPARAM,
) -> BOOL;

#[link(name = "user32")]
extern "system" {
    fn EnumDisplayMonitors(
        hdc: HDC,
        lprc_clip: *const RECT,
        lpfn_enum: MONITORENUMPROC,
        dw_data: LPARAM,
    ) -> BOOL;
    fn GetMonitorInfoW(hmonitor: HMONITOR, lpmi: *mut MONITORINFOEXW) -> BOOL;
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: String,
    pub is_primary: bool,
    pub monitor_rect: Rect,
    pub work_rect: Rect,
    pub scale: f32,
}

impl MonitorInfo {
    pub fn logical_work_rect(&self) -> Rect {
        let s = self.scale.max(0.5);
        Rect::from_min_max(self.work_rect.min / s, self.work_rect.max / s)
    }

    pub fn intersection_area(&self, r: &Rect) -> f32 {
        let is = self.monitor_rect.intersect(*r);
        if is.is_positive() {
            is.width() * is.height()
        } else {
            0.0
        }
    }
}

fn query_monitor_scale(hmon: HMONITOR) -> f32 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryA(lp_lib_file_name: *const std::ffi::c_char) -> *mut c_void;
        fn GetProcAddress(
            h_module: *mut c_void,
            lp_proc_name: *const std::ffi::c_char,
        ) -> *const c_void;
    }
    type GetDpiForMonitorFn = unsafe extern "system" fn(HMONITOR, i32, *mut u32, *mut u32) -> i32;
    static GET_DPI_FOR_MONITOR: std::sync::OnceLock<Option<GetDpiForMonitorFn>> =
        std::sync::OnceLock::new();

    let func_opt = GET_DPI_FOR_MONITOR.get_or_init(|| unsafe {
        let hmod = LoadLibraryA(c"shcore.dll".as_ptr());
        if hmod.is_null() {
            return None;
        }
        let proc = GetProcAddress(hmod, c"GetDpiForMonitor".as_ptr());
        if proc.is_null() {
            return None;
        }
        Some(std::mem::transmute::<*const c_void, GetDpiForMonitorFn>(proc))
    });

    if let Some(func) = func_opt {
        unsafe {
            let mut dpi_x: u32 = 96;
            let mut dpi_y: u32 = 96;
            if func(hmon, 0, &mut dpi_x, &mut dpi_y) == 0 && dpi_x >= 48 {
                return (dpi_x as f32 / 96.0).clamp(0.75, 4.0);
            }
        }
    }
    1.0
}

fn to_rect(r: &RECT) -> Rect {
    Rect::from_min_max(
        Pos2::new(r.left as f32, r.top as f32),
        Pos2::new(r.right as f32, r.bottom as f32),
    )
}

unsafe extern "system" fn enum_proc(
    hmon: HMONITOR,
    _hdc: HDC,
    _lprc: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let list = &mut *(lparam as *mut Vec<MonitorInfo>);
    let mut info: MONITORINFOEXW = std::mem::zeroed();
    info.cb_size = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(hmon, &mut info) != 0 {
        let len = info
            .sz_device
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.sz_device.len());
        let id = String::from_utf16_lossy(&info.sz_device[..len]);
        list.push(MonitorInfo {
            id,
            is_primary: (info.dw_flags & 1) != 0,
            monitor_rect: to_rect(&info.rc_monitor),
            work_rect: to_rect(&info.rc_work),
            scale: query_monitor_scale(hmon),
        });
    }
    1
}

/// Возвращает список всех подключенных мониторов с их физическими границами и рабочей областью (work area)
pub fn get_all_monitors() -> Vec<MonitorInfo> {
    let mut list = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            enum_proc,
            &mut list as *mut _ as LPARAM,
        );
    }
    list
}

pub fn find_monitor<'a>(monitors: &'a [MonitorInfo], id: &str) -> Option<&'a MonitorInfo> {
    monitors.iter().find(|m| m.id == id)
}

pub fn primary_or_first(monitors: &[MonitorInfo]) -> Option<&MonitorInfo> {
    monitors
        .iter()
        .find(|m| m.is_primary)
        .or_else(|| monitors.first())
}

pub fn best_monitor_for_rect<'a>(monitors: &'a [MonitorInfo], rect: &Rect) -> Option<&'a MonitorInfo> {
    monitors
        .iter()
        .max_by(|a, b| {
            let area_a = a.intersection_area(rect);
            let area_b = b.intersection_area(rect);
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .or_else(|| primary_or_first(monitors))
}

pub const SNAP_THRESHOLD: f32 = 7.0;

/// Прижимает координаты окна к краям рабочей зоны экрана (Desktop work area без панели задач),
/// если расстояние до края меньше или равно порогу threshold.
pub fn snap_to_work_area_with_threshold(
    mut pos: Pos2,
    size: Vec2,
    work: &Rect,
    threshold: f32,
    clamp_inside: bool,
) -> Pos2 {
    if threshold > 0.0 {
        let right = pos.x + size.x;
        let bottom = pos.y + size.y;

        let dl = (pos.x - work.left()).abs();
        let dr = (work.right() - right).abs();
        if dl <= dr && dl <= threshold {
            pos.x = work.left();
        } else if dr <= threshold {
            pos.x = work.right() - size.x;
        }

        let dt = (pos.y - work.top()).abs();
        let db = (work.bottom() - bottom).abs();
        if dt <= db && dt <= threshold {
            pos.y = work.top();
        } else if db <= threshold {
            pos.y = work.bottom() - size.y;
        }
    }

    if clamp_inside {
        let max_x = (work.right() - size.x).max(work.left());
        let max_y = (work.bottom() - size.y).max(work.top());
        pos.x = pos.x.clamp(work.left(), max_x);
        pos.y = pos.y.clamp(work.top(), max_y);
    }

    pos
}

/// Прижимает координаты окна к краям рабочей зоны экрана с порогом SNAP_THRESHOLD и зажимом в границы экрана.
pub fn snap_to_work_area(pos: Pos2, size: Vec2, work: &Rect) -> Pos2 {
    snap_to_work_area_with_threshold(pos, size, work, SNAP_THRESHOLD, true)
}

/// Вычисляет относительную позицию окна на мониторе (0.0 .. 1.0 по X и Y)
pub fn calculate_rel_pos(pos: Pos2, size: Vec2, work: &Rect) -> [f32; 2] {
    let avail_w = (work.width() - size.x).max(1.0);
    let avail_h = (work.height() - size.y).max(1.0);
    let rel_x = ((pos.x - work.left()) / avail_w).clamp(0.0, 1.0);
    let rel_y = ((pos.y - work.top()) / avail_h).clamp(0.0, 1.0);
    [rel_x, rel_y]
}

/// Восстанавливает абсолютную позицию окна на мониторе по относительным координатам с прижатием к краям
pub fn pos_from_rel(rel: [f32; 2], size: Vec2, work: &Rect) -> Pos2 {
    let avail_w = (work.width() - size.x).max(0.0);
    let avail_h = (work.height() - size.y).max(0.0);
    let x = work.left() + rel[0] * avail_w;
    let y = work.top() + rel[1] * avail_h;
    snap_to_work_area(Pos2::new(x, y), size, work)
}

pub fn place_child_in_work_area(
    work: &Rect,
    main_rect: Option<Rect>,
    child_size: Vec2,
    saved_pos: Option<[f32; 2]>,
) -> Option<Pos2> {
    // Запас под рамку заголовка Windows (~36px)
    let max_x = (work.right() - child_size.x - 4.0).max(work.left());
    let max_y = (work.bottom() - child_size.y - 36.0).max(work.top());

    if let Some([sx, sy]) = saved_pos {
        return Some(Pos2::new(sx.clamp(work.left(), max_x), sy.clamp(work.top(), max_y)));
    }

    let Some(main_r) = main_rect else {
        let cx = (work.center().x - child_size.x * 0.5).clamp(work.left(), max_x);
        let cy = (work.center().y - child_size.y * 0.5).clamp(work.top(), max_y);
        return Some(Pos2::new(cx, cy));
    };
    let gap = 14.0;
    let on_right_half = main_r.center().x >= work.center().x;
    let raw_x = if on_right_half {
        main_r.left() - child_size.x - gap
    } else {
        main_r.right() + gap
    };
    let raw_y = main_r.center().y - child_size.y * 0.5;

    Some(Pos2::new(
        raw_x.clamp(work.left(), max_x),
        raw_y.clamp(work.top(), max_y),
    ))
}

/// Рассчитывает позицию дочернего окна (Настройки / Редактирование):
/// - Если есть сохраненная позиция — зажимает её внутри видимой области ближайшего/основного монитора.
/// - Если позиции нет — размещает сбоку от главного окна со смещением к центру экрана.
pub fn place_child_near_main(
    main_rect: Option<Rect>,
    child_size: Vec2,
    saved_pos: Option<[f32; 2]>,
) -> Option<Pos2> {
    let monitors = get_all_monitors();
    let best_logical = |r: &Rect| {
        monitors.iter().max_by(|a, b| {
            let la = a.logical_work_rect().intersect(*r);
            let lb = b.logical_work_rect().intersect(*r);
            let aa = if la.is_positive() { la.width() * la.height() } else { 0.0 };
            let ab = if lb.is_positive() { lb.width() * lb.height() } else { 0.0 };
            aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal)
        })
    };

    let main_mon = main_rect
        .and_then(|r| best_logical(&r))
        .or_else(|| primary_or_first(&monitors))?;

    if let Some([sx, sy]) = saved_pos {
        let test_rect = Rect::from_min_size(Pos2::new(sx, sy), child_size);
        let mon_for_saved = best_logical(&test_rect).unwrap_or(main_mon);
        return place_child_in_work_area(
            &mon_for_saved.logical_work_rect(),
            main_rect,
            child_size,
            Some([sx, sy]),
        );
    }

    place_child_in_work_area(&main_mon.logical_work_rect(), main_rect, child_size, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_sticks_when_close_to_right_edge() {
        let work = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1920.0, 1040.0));
        let size = Vec2::new(140.0, 500.0);
        // 5px away from right edge (1920 - 140 = 1780 -> 1775 is 5px away <= 7px threshold)
        let snapped = snap_to_work_area(Pos2::new(1775.0, 5.0), size, &work);
        assert_eq!(snapped.x, 1780.0);
        assert_eq!(snapped.y, 0.0); // 5px <= 7px snap threshold from top
    }

    #[test]
    fn fallback_rel_preserves_right_dock() {
        let fallback_work = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(2560.0, 1392.0));
        let size = Vec2::new(140.0, 500.0);
        let pos = pos_from_rel([1.0, 0.0], size, &fallback_work);
        assert_eq!(pos, Pos2::new(2420.0, 0.0));
    }

    #[test]
    fn child_dialog_opens_toward_center_from_right_and_left_dock() {
        let work = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1920.0, 1040.0));
        let child_size = Vec2::new(360.0, 380.0);

        let right_main = Rect::from_min_size(Pos2::new(1770.0, 120.0), Vec2::new(140.0, 700.0));
        let pos_l = place_child_in_work_area(&work, Some(right_main), child_size, None).unwrap();
        assert_eq!(pos_l.x, 1770.0 - 360.0 - 14.0);

        let left_main = Rect::from_min_size(Pos2::new(10.0, 120.0), Vec2::new(140.0, 700.0));
        let pos_r = place_child_in_work_area(&work, Some(left_main), child_size, None).unwrap();
        assert_eq!(pos_r.x, 150.0 + 14.0);
    }

    #[test]
    fn secondary_monitor_bottom_snap_is_flush() {
        let work = Rect::from_min_max(Pos2::new(1920.0, 0.0), Pos2::new(3840.0, 1040.0));
        let size = Vec2::new(160.0, 500.0);
        // 5px above bottom (bottom = 1035 vs 1040) -> snaps cleanly flush to 1040 - 500 = 540
        let snapped = snap_to_work_area_with_threshold(
            Pos2::new(2200.0, 535.0),
            size,
            &work,
            SNAP_THRESHOLD,
            false,
        );
        assert_eq!(snapped, Pos2::new(2200.0, 540.0));
        // Outside threshold -> moves freely without clamping/sticking
        let free = snap_to_work_area_with_threshold(
            Pos2::new(2200.0, 500.0),
            size,
            &work,
            SNAP_THRESHOLD,
            false,
        );
        assert_eq!(free, Pos2::new(2200.0, 500.0));
    }
}
