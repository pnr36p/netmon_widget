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
}

impl MonitorInfo {
    pub fn contains_point(&self, p: Pos2) -> bool {
        self.monitor_rect.contains(p)
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

pub const SNAP_THRESHOLD: f32 = 20.0;

/// Прижимает координаты окна к краям рабочей зоны экрана (Desktop work area без панели задач),
/// если расстояние до края меньше или равно порогу SNAP_THRESHOLD (20px).
pub fn snap_to_work_area(mut pos: Pos2, size: Vec2, work: &Rect) -> Pos2 {
    let right = pos.x + size.x;
    let bottom = pos.y + size.y;

    // 1. Прижатие к левому краю
    if (pos.x - work.left()).abs() <= SNAP_THRESHOLD {
        pos.x = work.left();
    }
    // 2. Прижатие к правому краю
    if (work.right() - right).abs() <= SNAP_THRESHOLD {
        pos.x = work.right() - size.x;
    }
    // 3. Прижатие к верхнему краю
    if (pos.y - work.top()).abs() <= SNAP_THRESHOLD {
        pos.y = work.top();
    }
    // 4. Прижатие к нижнему краю
    if (work.bottom() - bottom).abs() <= SNAP_THRESHOLD {
        pos.y = work.bottom() - size.y;
    }

    // Защитный clamp, чтобы окно не вылезало за видимую область экрана
    let max_x = (work.right() - size.x).max(work.left());
    let max_y = (work.bottom() - size.y).max(work.top());
    pos.x = pos.x.clamp(work.left(), max_x);
    pos.y = pos.y.clamp(work.top(), max_y);

    pos
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
    let main_mon = main_rect
        .and_then(|r| best_monitor_for_rect(&monitors, &r))
        .or_else(|| primary_or_first(&monitors))?;

    if let Some([sx, sy]) = saved_pos {
        let test_rect = Rect::from_min_size(Pos2::new(sx, sy), child_size);
        let mon_for_saved = best_monitor_for_rect(&monitors, &test_rect).unwrap_or(main_mon);
        return place_child_in_work_area(&mon_for_saved.work_rect, main_rect, child_size, Some([sx, sy]));
    }

    place_child_in_work_area(&main_mon.work_rect, main_rect, child_size, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_sticks_when_close_to_right_edge() {
        let work = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1920.0, 1040.0));
        let size = Vec2::new(140.0, 500.0);
        // 12px away from right edge (1920 - 140 = 1780 -> 1768 is 12px away)
        let snapped = snap_to_work_area(Pos2::new(1768.0, 10.0), size, &work);
        assert_eq!(snapped.x, 1780.0);
        assert_eq!(snapped.y, 0.0); // 10px <= 20px snap threshold from top
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
}
