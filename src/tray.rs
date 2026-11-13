use crate::state::{AppState, OverallStatus, StatusSummary, TrayNotification, TrayNotifyLevel};
use std::sync::{Arc, Mutex};
use std::thread;

// ============================================================================
// Win32 FFI
// ============================================================================

type HWND = *mut std::ffi::c_void;
type HINSTANCE = *mut std::ffi::c_void;
type HMENU = *mut std::ffi::c_void;
type HICON = *mut std::ffi::c_void;
type HBITMAP = *mut std::ffi::c_void;
type HDC = *mut std::ffi::c_void;
type HANDLE = *mut std::ffi::c_void;
type UINT = u32;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type DWORD = u32;
type LPCWSTR = *const u16;
#[allow(non_camel_case_types)]
type UINT_PTR = usize;
type ATOM = u16;

const WM_TIMER: u32 = 0x0113;
const WM_APP: u32 = 0x8000;
const WM_DESTROY: u32 = 0x0002;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const WM_RBUTTONUP: u32 = 0x0205;

const WS_POPUP: DWORD = 0x8000_0000;
const WS_EX_TOOLWINDOW: DWORD = 0x0000_0080;

const NIM_ADD: u32 = 0x00000000;
const NIM_MODIFY: u32 = 0x00000001;
const NIM_DELETE: u32 = 0x00000002;

const NIF_MESSAGE: u32 = 0x00000001;
const NIF_ICON: u32 = 0x00000002;
const NIF_TIP: u32 = 0x00000004;
const NIF_INFO: u32 = 0x00000010;

const NIIF_INFO: u32 = 0x00000001;
const NIIF_WARNING: u32 = 0x00000002;
const NIIF_ERROR: u32 = 0x00000003;

#[link(name = "user32")]
extern "system" {
    fn DefWindowProcW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn RegisterClassExW(pcx: *const WNDCLASSEXW) -> ATOM;
    fn CreateWindowExW(
        dw_ex_style: DWORD,
        lp_class_name: LPCWSTR,
        lp_window_name: LPCWSTR,
        dw_style: DWORD,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        h_wnd_parent: HWND,
        h_menu: HMENU,
        h_instance: HINSTANCE,
        lp_param: *mut std::ffi::c_void,
    ) -> HWND;
    fn DestroyWindow(hwnd: HWND) -> i32;
    fn PostQuitMessage(n_exit_code: i32);
    fn GetMessageW(
        lp_msg: *mut MSG,
        hwnd: HWND,
        w_msg_filter_min: UINT,
        w_msg_filter_max: UINT,
    ) -> i32;
    fn TranslateMessage(lp_msg: *const MSG) -> i32;
    fn DispatchMessageW(lp_msg: *const MSG) -> LRESULT;
    fn PostMessageW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> i32;
    fn SetTimer(
        hwnd: HWND,
        n_id_event: UINT_PTR,
        u_elapse: UINT,
        lp_timer_func: *mut std::ffi::c_void,
    ) -> UINT_PTR;
    fn CreateIconIndirect(piconinfo: *const ICONINFO) -> HICON;
    fn DestroyIcon(h_icon: HICON) -> i32;
    fn GetLastError() -> DWORD;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateBitmap(
        width: i32,
        height: i32,
        planes: u32,
        bits_per_pixel: u32,
        lp_bits: *const std::ffi::c_void,
    ) -> HBITMAP;
    fn CreateDIBSection(
        hdc: HDC,
        pbmi: *const BITMAPINFO,
        usage: UINT,
        ppv_bits: *mut *mut std::ffi::c_void,
        h_section: HANDLE,
        offset: DWORD,
    ) -> HBITMAP;
    fn DeleteObject(h_object: HANDLE) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(dw_message: DWORD, lp_data: *mut NOTIFYICONDATAW) -> i32;
}

#[repr(C)]
struct WNDCLASSEXW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: usize,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HINSTANCE,
    h_icon: HICON,
    h_cursor: HDC,
    hbr_background: HDC,
    lpsz_menu_name: LPCWSTR,
    lpsz_class_name: LPCWSTR,
    h_icon_sm: HICON,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
    time: u32,
    pt_x: i32,
    pt_y: i32,
}

#[repr(C)]
struct ICONINFO {
    f_icon: i32,
    x_hotspot: u32,
    y_hotspot: u32,
    hbm_mask: HBITMAP,
    hbm_color: HBITMAP,
}

#[repr(C)]
struct BITMAPINFOHEADER {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct BITMAPINFO {
    bmi_header: BITMAPINFOHEADER,
    bmi_colors: [u32; 1],
}

#[repr(C)]
struct NOTIFYICONDATAW {
    cb_size: u32,
    h_wnd: HWND,
    u_id: u32,
    u_flags: u32,
    u_callback_message: u32,
    h_icon: HICON,
    sz_tip: [u16; 128],
    dw_state: u32,
    dw_state_mask: u32,
    sz_info: [u16; 256],
    u_timeout_or_version: u32,
    sz_info_title: [u16; 64],
    dw_flags: u32,
    guid_item: [u8; 16],
    h_balloon_icon: HICON,
}

// ============================================================================
// Процедурный векторный рендер иконки (для трея и окна приложения)
// ============================================================================

fn rounded_box_sdf(px: f32, py: f32, half_w: f32, half_h: f32, r: f32) -> f32 {
    let qx = px.abs() - (half_w - r);
    let qy = py.abs() - (half_h - r);
    let out_x = qx.max(0.0);
    let out_y = qy.max(0.0);
    (out_x * out_x + out_y * out_y).sqrt() + qx.max(qy).min(0.0) - r
}

fn dist_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let wx = px - ax;
    let wy = py - ay;
    let len_sq = vx * vx + vy * vy;
    let t = if len_sq > 1e-6 {
        ((wx * vx + wy * vy) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let dx = px - (ax + t * vx);
    let dy = py - (ay + t * vy);
    (dx * dx + dy * dy).sqrt()
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

fn blend(dst: [f32; 4], src: [f32; 4]) -> [f32; 4] {
    let sa = src[3];
    let inv = 1.0 - sa;
    [
        src[0] * sa + dst[0] * inv,
        src[1] * sa + dst[1] * inv,
        src[2] * sa + dst[2] * inv,
        (sa + dst[3] * inv).min(1.0),
    ]
}

fn sample_icon_pixel(u: f32, v: f32, status: OverallStatus) -> [f32; 4] {
    let mut col = [0.0f32; 4];

    // 1. Тёмная плашка со скругленными углами и металлической окантовкой
    let sd_box = rounded_box_sdf(u - 0.5, v - 0.5, 0.45, 0.45, 0.16);
    if sd_box <= 0.02 {
        // Тело плашки (градиент тёмно-синего графита + лёгкий глянец сверху)
        let bg_r = lerp_u8(0x23, 0x13, v) as f32 / 255.0;
        let bg_g = lerp_u8(0x33, 0x1D, v) as f32 / 255.0;
        let bg_b = lerp_u8(0x49, 0x2B, v) as f32 / 255.0;
        let gloss = if v < 0.46 { (0.46 - v) * 0.25 } else { 0.0 };

        let border_t = if sd_box > -0.055 { 1.0 } else { 0.0 };
        let br_r = lerp_u8(0x88, 0x45, v) as f32 / 255.0;
        let br_g = lerp_u8(0xBF, 0x76, v) as f32 / 255.0;
        let br_b = lerp_u8(0xF8, 0xB0, v) as f32 / 255.0;

        let alpha = ((-sd_box) / 0.025 + 0.5).clamp(0.0, 1.0);
        let r = (bg_r + gloss) * (1.0 - border_t) + br_r * border_t;
        let g = (bg_g + gloss) * (1.0 - border_t) + br_g * border_t;
        let b = (bg_b + gloss * 1.1) * (1.0 - border_t) + br_b * border_t;
        col = [r.min(1.0), g.min(1.0), b.min(1.0), alpha];
    }

    // 2. Три восходящих столбика сетевого сигнала (фирменный светло-голубой)
    let bars = [
        (0.245, 0.65, 0.055, 0.12),
        (0.425, 0.565, 0.055, 0.205),
        (0.605, 0.48, 0.055, 0.29),
    ];
    for &(cx, cy, hw, hh) in &bars {
        let sd_bar = rounded_box_sdf(u - cx, v - cy, hw, hh, 0.028);
        if sd_bar < 0.015 {
            let a = ((-sd_bar) / 0.02 + 0.5).clamp(0.0, 1.0);
            let top_glow = (1.0 - (v - (cy - hh)) / (2.0 * hh).max(0.01)).clamp(0.0, 1.0);
            let bar_col = [
                0.35 + top_glow * 0.28,
                0.73 + top_glow * 0.18,
                1.0,
                a,
            ];
            col = blend(col, bar_col);
        }
    }

    // 3. Контрастный бейдж статуса в правом нижнем углу
    let bx = 0.73;
    let by = 0.73;
    let dist_badge = ((u - bx) * (u - bx) + (v - by) * (v - by)).sqrt();

    if dist_badge < 0.25 {
        // Тёмный вырез/ободок для читаемости на любом фоне
        let ring_a = ((0.24 - dist_badge) / 0.02).clamp(0.0, 1.0);
        col = blend(col, [0.07, 0.10, 0.15, ring_a]);
    }

    if dist_badge < 0.205 {
        let dot_a = ((0.195 - dist_badge) / 0.02).clamp(0.0, 1.0);
        let grad = ((by - v + 0.18) / 0.36).clamp(0.0, 1.0);
        let base_col = match status {
            OverallStatus::Ok => [0.18 + grad * 0.15, 0.80 + grad * 0.16, 0.22 + grad * 0.15, dot_a],
            OverallStatus::Pending => [0.95, 0.68 + grad * 0.22, 0.05 + grad * 0.15, dot_a],
            OverallStatus::Error => [0.95 + grad * 0.05, 0.14 + grad * 0.15, 0.14 + grad * 0.12, dot_a],
        };
        col = blend(col, base_col);

        // Внутренний белый символ внутри бейджа
        let sym_d = match status {
            OverallStatus::Ok => {
                let d1 = dist_segment(u, v, bx - 0.085, by + 0.005, bx - 0.02, by + 0.07);
                let d2 = dist_segment(u, v, bx - 0.02, by + 0.07, bx + 0.09, by - 0.065);
                d1.min(d2)
            }
            OverallStatus::Pending => {
                ((u - bx) * (u - bx) + (v - by) * (v - by)).sqrt() - 0.025
            }
            OverallStatus::Error => {
                let d1 = dist_segment(u, v, bx - 0.07, by - 0.07, bx + 0.07, by + 0.07);
                let d2 = dist_segment(u, v, bx + 0.07, by - 0.07, bx - 0.07, by + 0.07);
                d1.min(d2)
            }
        };
        let stroke_r = if status == OverallStatus::Pending { 0.03 } else { 0.032 };
        if sym_d < stroke_r + 0.015 {
            let sym_a = ((stroke_r - sym_d) / 0.02 + 0.5).clamp(0.0, 1.0);
            col = blend(col, [1.0, 1.0, 1.0, sym_a]);
        }
    }

    col
}

/// Генерирует чёткую сглаженную (2x2 SSAA) иконку RGBA для окна и системного трея.
pub fn render_app_icon_rgba(status: OverallStatus, size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    let inv = 1.0 / size as f32;
    let offsets = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];

    for y in 0..size {
        for x in 0..size {
            let mut acc = [0.0f32; 4];
            for &(ox, oy) in &offsets {
                let u = (x as f32 + ox) * inv;
                let v = (y as f32 + oy) * inv;
                let s = sample_icon_pixel(u, v, status);
                acc[0] += s[0] * s[3];
                acc[1] += s[1] * s[3];
                acc[2] += s[2] * s[3];
                acc[3] += s[3];
            }
            let a = acc[3] * 0.25;
            if a > 1e-4 {
                let r = ((acc[0] * 0.25 / a).clamp(0.0, 1.0) * 255.0).round() as u8;
                let g = ((acc[1] * 0.25 / a).clamp(0.0, 1.0) * 255.0).round() as u8;
                let b = ((acc[2] * 0.25 / a).clamp(0.0, 1.0) * 255.0).round() as u8;
                let a8 = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
                pixels.extend_from_slice(&[r, g, b, a8]);
            } else {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    pixels
}

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn format_tray_tooltip(summary: &StatusSummary, lang: crate::i18n::Language) -> String {
    let s = crate::i18n::tr(lang);
    match summary.overall {
        OverallStatus::Ok => {
            if summary.optional_down_count > 0 {
                let body = s
                    .tray_online_opt
                    .replacen("{}", &summary.online_count.to_string(), 1)
                    .replacen("{}", &summary.total_count.to_string(), 1)
                    .replacen("{}", &summary.optional_down_count.to_string(), 1);
                format!("NetMon Widget — {}", body)
            } else {
                let body = s
                    .tray_all_online
                    .replacen("{}", &summary.online_count.to_string(), 1)
                    .replacen("{}", &summary.total_count.to_string(), 1);
                format!("NetMon Widget — {}", body)
            }
        }
        OverallStatus::Pending => {
            let body = s
                .tray_checking
                .replacen("{}", &summary.online_count.to_string(), 1)
                .replacen("{}", &summary.total_count.to_string(), 1);
            format!("NetMon Widget — {}", body)
        }
        OverallStatus::Error => {
            let count = summary.down_critical_names.len();
            let preview: Vec<&str> = summary
                .down_critical_names
                .iter()
                .take(3)
                .map(|n| n.as_str())
                .collect();
            let suffix = if count > 3 { ", …" } else { "" };
            let prefix = s.tray_error.replacen("{}", &count.to_string(), 1);
            format!("NetMon Widget — {}{}{}", prefix, preview.join(", "), suffix)
        }
    }
}

fn write_wide_buf<const N: usize>(buf: &mut [u16; N], text: &str) {
    buf.fill(0);
    if N == 0 {
        return;
    }
    for (i, code) in text.encode_utf16().take(N - 1).enumerate() {
        buf[i] = code;
    }
}

unsafe fn show_balloon(hwnd: HWND, notif: &TrayNotification) {
    let mut nid = NOTIFYICONDATAW {
        cb_size: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        h_wnd: hwnd,
        u_id: 1,
        u_flags: NIF_INFO,
        u_callback_message: WM_APP + 1,
        h_icon: std::ptr::null_mut(),
        sz_tip: [0u16; 128],
        dw_state: 0,
        dw_state_mask: 0,
        sz_info: [0u16; 256],
        u_timeout_or_version: 5000,
        sz_info_title: [0u16; 64],
        dw_flags: match notif.level {
            TrayNotifyLevel::Error => NIIF_ERROR,
            TrayNotifyLevel::Warning => NIIF_WARNING,
            TrayNotifyLevel::Info => NIIF_INFO,
        },
        guid_item: [0u8; 16],
        h_balloon_icon: std::ptr::null_mut(),
    };
    write_wide_buf(&mut nid.sz_info_title, &notif.title);
    write_wide_buf(&mut nid.sz_info, &notif.body);
    let _ = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
}

// ============================================================================
// Трей
// ============================================================================

#[derive(Debug)]
pub enum TrayCommand {
    Menu(crate::ui::menu::native::NativeMenuAction),
}

pub fn spawn_tray(
    state: Arc<Mutex<AppState>>,
    tx: std::sync::mpsc::Sender<TrayCommand>,
    egui_ctx: Arc<Mutex<Option<eframe::egui::Context>>>,
) {
    thread::spawn(move || unsafe {
        run_tray(state, tx, egui_ctx);
    });
}

unsafe fn create_hicon_from_rgba(rgba: &[u8], size: u32) -> HICON {
    let bmi = BITMAPINFO {
        bmi_header: BITMAPINFOHEADER {
            bi_size: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            bi_width: size as i32,
            bi_height: -(size as i32), // top-down DIB
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: 0, // BI_RGB
            bi_size_image: 0,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        },
        bmi_colors: [0],
    };

    let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let hbm_color = CreateDIBSection(
        std::ptr::null_mut(),
        &bmi,
        0,
        &mut bits_ptr,
        std::ptr::null_mut(),
        0,
    );
    if hbm_color.is_null() || bits_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let pixel_count = (size * size) as usize;
    let dst = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, pixel_count * 4);
    let stride_bytes = ((size + 31) / 32 * 4) as usize;
    let mut mask_bits = vec![0u8; stride_bytes * size as usize];

    for y in 0..size as usize {
        for x in 0..size as usize {
            let idx = (y * size as usize + x) * 4;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];

            // Windows 32-bit DIB для альфа-иконок: BGRA
            dst[idx] = b;
            dst[idx + 1] = g;
            dst[idx + 2] = r;
            dst[idx + 3] = a;

            if a < 16 {
                let byte_idx = y * stride_bytes + (x / 8);
                mask_bits[byte_idx] |= 0x80 >> (x % 8);
            }
        }
    }

    let hbm_mask = CreateBitmap(
        size as i32,
        size as i32,
        1,
        1,
        mask_bits.as_ptr() as *const std::ffi::c_void,
    );
    if hbm_mask.is_null() {
        DeleteObject(hbm_color);
        return std::ptr::null_mut();
    }

    let icon_info = ICONINFO {
        f_icon: 1,
        x_hotspot: 0,
        y_hotspot: 0,
        hbm_mask,
        hbm_color,
    };

    let hicon = CreateIconIndirect(&icon_info);
    DeleteObject(hbm_color);
    DeleteObject(hbm_mask);
    hicon
}

unsafe fn run_tray(
    state: Arc<Mutex<AppState>>,
    tx: std::sync::mpsc::Sender<TrayCommand>,
    egui_ctx: Arc<Mutex<Option<eframe::egui::Context>>>,
) {
    let class_name = encode_wide("NetMonTrayClass");

    let wc = WNDCLASSEXW {
        cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfn_wnd_proc: window_proc as *const () as usize,
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: std::ptr::null_mut(),
        h_icon: std::ptr::null_mut(),
        h_cursor: std::ptr::null_mut(),
        hbr_background: std::ptr::null_mut(),
        lpsz_menu_name: std::ptr::null(),
        lpsz_class_name: class_name.as_ptr(),
        h_icon_sm: std::ptr::null_mut(),
    };
    let _ = RegisterClassExW(&wc);

    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        class_name.as_ptr(),
        encode_wide("NetMonTray").as_ptr(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        eprintln!("tray: cannot create window");
        return;
    }

    let init_rgba = render_app_icon_rgba(OverallStatus::Pending, 32);
    let mut current_icon: HICON = create_hicon_from_rgba(&init_rgba, 32);

    let mut nid = NOTIFYICONDATAW {
        cb_size: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        h_wnd: hwnd,
        u_id: 1,
        u_flags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        u_callback_message: WM_APP + 1,
        h_icon: current_icon,
        sz_tip: [0u16; 128],
        dw_state: 0,
        dw_state_mask: 0,
        sz_info: [0u16; 256],
        u_timeout_or_version: 0,
        sz_info_title: [0u16; 64],
        dw_flags: 0,
        guid_item: [0u8; 16],
        h_balloon_icon: std::ptr::null_mut(),
    };
    write_wide_buf(&mut nid.sz_tip, "NetMon Widget — Запуск…");
    Shell_NotifyIconW(NIM_ADD, &mut nid);

    let mut apply_tray = |summary: &StatusSummary, lang: crate::i18n::Language, update_icon: bool| {
        if update_icon {
            let rgba = render_app_icon_rgba(summary.overall, 32);
            let hicon = create_hicon_from_rgba(&rgba, 32);
            if hicon.is_null() {
                eprintln!("tray: CreateIconIndirect failed, error={}", GetLastError());
                return;
            }
            if !current_icon.is_null() {
                let _ = DestroyIcon(current_icon);
            }
            current_icon = hicon;
        }

        let tip_text = format_tray_tooltip(summary, lang);
        let mut mod_nid = NOTIFYICONDATAW {
            cb_size: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            h_wnd: hwnd,
            u_id: 1,
            u_flags: if update_icon { NIF_ICON | NIF_TIP } else { NIF_TIP },
            u_callback_message: WM_APP + 1,
            h_icon: current_icon,
            sz_tip: [0u16; 128],
            dw_state: 0,
            dw_state_mask: 0,
            sz_info: [0u16; 256],
            u_timeout_or_version: 0,
            sz_info_title: [0u16; 64],
            dw_flags: 0,
            guid_item: [0u8; 16],
            h_balloon_icon: std::ptr::null_mut(),
        };
        write_wide_buf(&mut mod_nid.sz_tip, &tip_text);
        let _ = Shell_NotifyIconW(NIM_MODIFY, &mut mod_nid);
    };

    SetTimer(hwnd, 1, 400, std::ptr::null_mut());

    let mut last_summary: Option<StatusSummary> = None;
    let mut last_lang: Option<crate::i18n::Language> = None;

    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        if msg.message == WM_TIMER {
            let (summary, lang, next_notif) = {
                let mut st = state.lock().unwrap();
                (
                    st.status_summary(),
                    st.config.language,
                    st.pending_notifications.pop_front(),
                )
            };
            if last_summary.as_ref() != Some(&summary) || last_lang != Some(lang) {
                let icon_changed = last_summary
                    .as_ref()
                    .map(|s| s.overall != summary.overall)
                    .unwrap_or(true);
                apply_tray(&summary, lang, icon_changed);
                last_summary = Some(summary);
                last_lang = Some(lang);
            }
            if let Some(notif) = next_notif {
                show_balloon(hwnd, &notif);
            }
            continue;
        }

        if msg.message == WM_APP + 1 || msg.message == WM_APP + 2 {
            let lparam = msg.l_param as u32;
            if lparam == WM_LBUTTONUP || lparam == WM_RBUTTONUP {
                let (aot, minimized, hide_taskbar, lang, is_dark, interval_sec) = {
                    let st = state.lock().unwrap();
                    (
                        st.config.always_on_top,
                        st.minimized_to_tray,
                        st.config.hide_taskbar,
                        st.config.language,
                        st.config.theme.resolve_is_dark(),
                        st.config.interval_sec,
                    )
                };
                if let Some(act) = crate::ui::menu::native::show_popup(
                    lang,
                    is_dark,
                    aot,
                    minimized,
                    hide_taskbar,
                    interval_sec,
                    crate::ui::menu::native::MenuTarget::Tray,
                ) {
                    let _ = tx.send(TrayCommand::Menu(act));
                    if let Some(c) = egui_ctx.lock().unwrap().as_ref() {
                        c.request_repaint();
                    }
                }
            } else if lparam == WM_LBUTTONDBLCLK {
                let minimized = state.lock().unwrap().minimized_to_tray;
                let act = if minimized {
                    crate::ui::menu::native::NativeMenuAction::ToggleMinimize
                } else {
                    crate::ui::menu::native::NativeMenuAction::Settings
                };
                let _ = tx.send(TrayCommand::Menu(act));
                if let Some(c) = egui_ctx.lock().unwrap().as_ref() {
                    c.request_repaint();
                }
            }
            continue;
        }

        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    if !current_icon.is_null() {
        let _ = DestroyIcon(current_icon);
    }
    let _ = Shell_NotifyIconW(NIM_DELETE, &mut nid);
    DestroyWindow(hwnd);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        m if m == WM_APP + 1 => {
            PostMessageW(hwnd, WM_APP + 2, w_param, l_param);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}
