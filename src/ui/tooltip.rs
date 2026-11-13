#![allow(clippy::upper_case_acronyms)]

use crate::history::{UptimeWindows, WindowUptime};
use crate::i18n::{tr, Language};
use crate::state::CheckState;
use eframe::egui::{Pos2, Rect};

#[derive(Debug, Clone, PartialEq)]
struct TooltipContent {
    title_line: String,
    optional_tag: bool,
    target_line: String,
    status_line: String,
    status_rgb: (u8, u8, u8),
    error_line: Option<String>,
    stats_line: Option<String>,
    uptime_line: Option<String>,
}

fn fmt_window_pct(label: &str, w: WindowUptime) -> String {
    match w.percent() {
        Some(p) if (p - 100.0).abs() < 0.05 => format!("{}: 100%", label),
        Some(p) => format!("{}: {:.1}%", label, p),
        None => format!("{}: —", label),
    }
}

fn build_content(
    check: &CheckState,
    windows: UptimeWindows,
    lang: Language,
    is_dark: bool,
) -> TooltipContent {
    let s = tr(lang);
    let optional_tag = check.config.is_optional();
    let title_line = if optional_tag {
        format!("{}  [{}]", check.config.name, s.tt_non_critical_badge)
    } else {
        check.config.name.clone()
    };

    let proto = check.config.base_kind().to_uppercase();
    let eff_port = check.config.effective_port();
    let target_str = if eff_port > 0 && check.config.base_kind() != "ping" {
        if check.config.base_kind() == "dns" && check.config.port.is_none() {
            check.config.host.clone()
        } else {
            format!("{}:{}", check.config.host, eff_port)
        }
    } else {
        check.config.host.clone()
    };
    let target_line = format!("{}: {} • {}", s.tt_target, target_str, proto);

    let session_part = if check.total_count > 0 {
        let pct = check.ok_count as f32 / check.total_count as f32 * 100.0;
        let pct_str = if (pct - 100.0).abs() < 0.05 {
            "100%".to_string()
        } else {
            format!("{:.1}%", pct)
        };
        format!(
            "{}: {} ({}/{})",
            s.tt_uptime_session, pct_str, check.ok_count, check.total_count
        )
    } else {
        format!("{}: —", s.tt_uptime_session)
    };

    let uptime_line = if check.total_count > 0 || windows.month.total > 0 {
        Some(format!(
            "{} • {} • {} • {}",
            session_part,
            fmt_window_pct(s.tt_day, windows.day),
            fmt_window_pct(s.tt_week, windows.week),
            fmt_window_pct(s.tt_month, windows.month),
        ))
    } else {
        None
    };

    match &check.result {
        Some(res) if res.ok => {
            let ms_str = res
                .latency_ms
                .map(|ms| {
                    if ms == 0 {
                        format!("<1 {}", s.tt_ms)
                    } else {
                        format!("{} {}", ms, s.tt_ms)
                    }
                })
                .unwrap_or_else(|| "OK".into());
            TooltipContent {
                title_line,
                optional_tag,
                target_line,
                status_line: format!("{} ({})", s.tt_status_online, ms_str),
                status_rgb: if is_dark {
                    (0x4E, 0xE0, 0x58)
                } else {
                    (0x15, 0x80, 0x3D)
                },
                error_line: None,
                stats_line: Some(format!(
                    "{}: {}",
                    s.tt_checked,
                    res.timestamp.format("%H:%M:%S")
                )),
                uptime_line,
            }
        }
        Some(res) => {
            let (status_line, status_rgb) = if optional_tag {
                (
                    s.tt_status_opt_down.to_string(),
                    if is_dark {
                        (0xFF, 0xD2, 0x28)
                    } else {
                        (0xB4, 0x53, 0x09)
                    },
                )
            } else {
                (
                    s.tt_status_offline.to_string(),
                    if is_dark {
                        (0xFF, 0x5A, 0x5A)
                    } else {
                        (0xDC, 0x26, 0x26)
                    },
                )
            };
            let error_line = res
                .error
                .as_ref()
                .filter(|e| !e.trim().is_empty())
                .map(|e| format!("{}: {}", s.tt_reason, e.trim()));

            let last_ok = check
                .last_ok_at
                .map(|t| format!(" • {}: {}", s.tt_last_online, t.format("%H:%M:%S")))
                .unwrap_or_default();

            TooltipContent {
                title_line,
                optional_tag,
                target_line,
                status_line,
                status_rgb,
                error_line,
                stats_line: Some(format!(
                    "{}: {}{}",
                    s.tt_checked,
                    res.timestamp.format("%H:%M:%S"),
                    last_ok
                )),
                uptime_line,
            }
        }
        None => TooltipContent {
            title_line,
            optional_tag,
            target_line,
            status_line: s.tt_status_pending.to_string(),
            status_rgb: (0xAA, 0xB2, 0xBD),
            error_line: None,
            stats_line: None,
            uptime_line,
        },
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use std::cell::RefCell;
    use std::ffi::c_void;

    type HWND = *mut c_void;
    type HINSTANCE = *mut c_void;
    type HMENU = *mut c_void;
    type HDC = *mut c_void;
    type HFONT = *mut c_void;
    type HBRUSH = *mut c_void;
    type HGDIOBJ = *mut c_void;
    type BOOL = i32;
    type UINT = u32;
    type DWORD = u32;
    type WPARAM = usize;
    type LPARAM = isize;
    type LRESULT = isize;
    type LPCWSTR = *const u16;
    type ATOM = u16;

    const WS_POPUP: DWORD = 0x80000000;
    const WS_EX_TOPMOST: DWORD = 0x00000008;
    const WS_EX_TRANSPARENT: DWORD = 0x00000020;
    const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
    const WS_EX_NOACTIVATE: DWORD = 0x08000000;
    const CS_DROPSHADOW: UINT = 0x00020000;

    const SW_HIDE: i32 = 0;
    const SWP_NOACTIVATE: UINT = 0x0010;
    const SWP_SHOWWINDOW: UINT = 0x0040;
    const HWND_TOPMOST: HWND = -1isize as HWND;

    const WM_PAINT: UINT = 0x000F;
    const WM_ERASEBKGND: UINT = 0x0014;
    const WM_NCHITTEST: UINT = 0x0084;
    const HTTRANSPARENT: LRESULT = -1;

    const TRANSPARENT: i32 = 1;
    const DT_LEFT: UINT = 0x00000000;
    const DT_WORDBREAK: UINT = 0x00000010;
    const DT_CALCRECT: UINT = 0x00000400;
    const DT_NOPREFIX: UINT = 0x00000800;

    const FW_NORMAL: i32 = 400;
    const FW_SEMIBOLD: i32 = 600;
    const DEFAULT_CHARSET: DWORD = 1;
    const CLEARTYPE_QUALITY: DWORD = 5;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct PAINTSTRUCT {
        hdc: HDC,
        f_erase: BOOL,
        rc_paint: RECT,
        f_restore: BOOL,
        f_inc_update: BOOL,
        rgb_reserved: [u8; 32],
    }

    #[repr(C)]
    struct WNDCLASSEXW {
        cb_size: u32,
        style: u32,
        lpfn_wnd_proc: usize,
        cb_cls_extra: i32,
        cb_wnd_extra: i32,
        h_instance: HINSTANCE,
        h_icon: *mut c_void,
        h_cursor: *mut c_void,
        hbr_background: HBRUSH,
        lpsz_menu_name: LPCWSTR,
        lpsz_class_name: LPCWSTR,
        h_icon_sm: *mut c_void,
    }

    #[link(name = "user32")]
    extern "system" {
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
            lp_param: *mut c_void,
        ) -> HWND;
        fn DefWindowProcW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
        fn ShowWindow(hwnd: HWND, n_cmd_show: i32) -> BOOL;
        fn SetWindowPos(
            hwnd: HWND,
            hwnd_insert_after: HWND,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: UINT,
        ) -> BOOL;
        fn InvalidateRect(hwnd: HWND, lp_rect: *const RECT, b_erase: BOOL) -> BOOL;
        fn BeginPaint(hwnd: HWND, lp_paint: *mut PAINTSTRUCT) -> HDC;
        fn EndPaint(hwnd: HWND, lp_paint: *const PAINTSTRUCT) -> BOOL;
        fn GetDC(hwnd: HWND) -> HDC;
        fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
        fn FillRect(hdc: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
        fn FrameRect(hdc: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
        fn DrawTextW(
            hdc: HDC,
            lpch_text: LPCWSTR,
            cch_text: i32,
            lprc: *mut RECT,
            format: UINT,
        ) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateSolidBrush(color: DWORD) -> HBRUSH;
        fn DeleteObject(ho: HGDIOBJ) -> BOOL;
        fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
        fn SetBkMode(hdc: HDC, mode: i32) -> i32;
        fn SetTextColor(hdc: HDC, color: DWORD) -> DWORD;
        fn CreateFontW(
            c_height: i32,
            c_width: i32,
            c_escapement: i32,
            c_orientation: i32,
            c_weight: i32,
            b_italic: DWORD,
            b_underline: DWORD,
            b_strike_out: DWORD,
            i_char_set: DWORD,
            i_out_precision: DWORD,
            i_clip_precision: DWORD,
            i_quality: DWORD,
            i_pitch_and_family: DWORD,
            psz_face_name: LPCWSTR,
        ) -> HFONT;
    }

    #[inline]
    fn rgb(r: u8, g: u8, b: u8) -> DWORD {
        (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
    }

    fn encode_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    struct TooltipState {
        hwnd: HWND,
        visible: bool,
        content: Option<TooltipContent>,
        scale: f32,
        is_dark: bool,
        last_rect: (i32, i32, i32, i32),
        last_anchor: (i32, i32, i32, i32),
    }

    thread_local! {
        static TIP: RefCell<TooltipState> = const {
            RefCell::new(TooltipState {
                hwnd: std::ptr::null_mut(),
                visible: false,
                content: None,
                scale: 1.0,
                is_dark: true,
                last_rect: (0, 0, 0, 0),
                last_anchor: (0, 0, 0, 0),
            })
        };
    }

    unsafe fn make_font(height_px: i32, bold: bool) -> HFONT {
        let face = encode_wide("Segoe UI");
        CreateFontW(
            -height_px,
            0,
            0,
            0,
            if bold { FW_SEMIBOLD } else { FW_NORMAL },
            0,
            0,
            0,
            DEFAULT_CHARSET,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            face.as_ptr(),
        )
    }

    unsafe fn measure_line(hdc: HDC, font: HFONT, text: &str, max_w: i32) -> (i32, i32) {
        let old = SelectObject(hdc, font as HGDIOBJ);
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: max_w,
            bottom: 0,
        };
        DrawTextW(
            hdc,
            wide.as_ptr(),
            wide.len() as i32,
            &mut rc,
            DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX | DT_LEFT,
        );
        SelectObject(hdc, old);
        (rc.right - rc.left, rc.bottom - rc.top)
    }

    unsafe fn draw_line(hdc: HDC, font: HFONT, color: DWORD, text: &str, mut rc: RECT) {
        let old = SelectObject(hdc, font as HGDIOBJ);
        SetTextColor(hdc, color);
        let wide: Vec<u16> = text.encode_utf16().collect();
        DrawTextW(
            hdc,
            wide.as_ptr(),
            wide.len() as i32,
            &mut rc,
            DT_WORDBREAK | DT_NOPREFIX | DT_LEFT,
        );
        SelectObject(hdc, old);
    }

    unsafe fn compute_layout(hwnd: HWND, c: &TooltipContent, scale: f32) -> (i32, i32) {
        let hdc = GetDC(hwnd);
        let pad_x = (10.0 * scale).round() as i32;
        let pad_y = (8.0 * scale).round() as i32;
        let gap = (3.0 * scale).round() as i32;
        let max_text_w = (340.0 * scale).round() as i32;

        let font_title = make_font((13.0 * scale).round() as i32, true);
        let font_body = make_font((11.5 * scale).round() as i32, false);
        let font_small = make_font((10.5 * scale).round() as i32, false);

        let (tw, th) = measure_line(hdc, font_title, &c.title_line, max_text_w);
        let (tgw, tgh) = measure_line(hdc, font_body, &c.target_line, max_text_w);
        let (sw, sh) = measure_line(hdc, font_body, &c.status_line, max_text_w);

        let mut content_w = tw.max(tgw).max(sw);
        let mut content_h = th + gap + tgh + gap + sh;

        if let Some(err) = &c.error_line {
            let (ew, eh) = measure_line(hdc, font_small, err, max_text_w);
            content_w = content_w.max(ew);
            content_h += gap + eh;
        }
        if let Some(st) = &c.stats_line {
            let (fw, fh) = measure_line(hdc, font_small, st, max_text_w);
            content_w = content_w.max(fw);
            content_h += gap + fh;
        }
        if let Some(up) = &c.uptime_line {
            let (uw, uh) = measure_line(hdc, font_small, up, max_text_w);
            content_w = content_w.max(uw);
            content_h += gap + uh;
        }

        DeleteObject(font_title as HGDIOBJ);
        DeleteObject(font_body as HGDIOBJ);
        DeleteObject(font_small as HGDIOBJ);
        ReleaseDC(hwnd, hdc);

        (content_w + pad_x * 2, content_h + pad_y * 2)
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: UINT,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_NCHITTEST => HTTRANSPARENT,
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                TIP.with(|cell| {
                    let st = cell.borrow();
                    let scale = st.scale;
                    let is_dark = st.is_dark;
                    let w = st.last_rect.2;
                    let h = st.last_rect.3;

                    let (bg_c, border_c, title_c, target_c, err_opt_c, err_crit_c, stats_c, up_c) =
                        if is_dark {
                            (
                                rgb(0x26, 0x26, 0x28),
                                rgb(0x6E, 0x76, 0x82),
                                rgb(0xFF, 0xFF, 0xFF),
                                rgb(0xB8, 0xC2, 0xCE),
                                rgb(0xE8, 0xCA, 0x72),
                                rgb(0xFF, 0x94, 0x94),
                                rgb(0x8D, 0x97, 0xA5),
                                rgb(0xA8, 0xB8, 0xCA),
                            )
                        } else {
                            (
                                rgb(0xFA, 0xFB, 0xFD),
                                rgb(0xB0, 0xBA, 0xC6),
                                rgb(0x16, 0x1B, 0x22),
                                rgb(0x47, 0x55, 0x69),
                                rgb(0xB4, 0x53, 0x09),
                                rgb(0xDC, 0x26, 0x26),
                                rgb(0x52, 0x60, 0x71),
                                rgb(0x3B, 0x4A, 0x5E),
                            )
                        };

                    let bg_brush = CreateSolidBrush(bg_c);
                    let border_brush = CreateSolidBrush(border_c);
                    let full = RECT {
                        left: 0,
                        top: 0,
                        right: w,
                        bottom: h,
                    };
                    FillRect(hdc, &full, bg_brush);
                    FrameRect(hdc, &full, border_brush);
                    DeleteObject(bg_brush as HGDIOBJ);
                    DeleteObject(border_brush as HGDIOBJ);

                    if let Some(c) = &st.content {
                        SetBkMode(hdc, TRANSPARENT);
                        let pad_x = (10.0 * scale).round() as i32;
                        let mut y = (8.0 * scale).round() as i32;
                        let gap = (3.0 * scale).round() as i32;
                        let text_w = (w - pad_x * 2).max(20);

                        let font_title = make_font((13.0 * scale).round() as i32, true);
                        let font_body = make_font((11.5 * scale).round() as i32, false);
                        let font_small = make_font((10.5 * scale).round() as i32, false);

                        let line_rect = |top: i32, h: i32| RECT {
                            left: pad_x,
                            top,
                            right: pad_x + text_w,
                            bottom: top + h,
                        };

                        let (_, th) = measure_line(hdc, font_title, &c.title_line, text_w);
                        draw_line(
                            hdc,
                            font_title,
                            title_c,
                            &c.title_line,
                            line_rect(y, th),
                        );
                        y += th + gap;

                        let (_, tgh) = measure_line(hdc, font_body, &c.target_line, text_w);
                        draw_line(
                            hdc,
                            font_body,
                            target_c,
                            &c.target_line,
                            line_rect(y, tgh),
                        );
                        y += tgh + gap;

                        let (_, sh) = measure_line(hdc, font_body, &c.status_line, text_w);
                        let (sr, sg, sb) = c.status_rgb;
                        draw_line(
                            hdc,
                            font_body,
                            rgb(sr, sg, sb),
                            &c.status_line,
                            line_rect(y, sh),
                        );
                        y += sh + gap;

                        if let Some(err) = &c.error_line {
                            let (_, eh) = measure_line(hdc, font_small, err, text_w);
                            let err_color = if c.optional_tag { err_opt_c } else { err_crit_c };
                            draw_line(hdc, font_small, err_color, err, line_rect(y, eh));
                            y += eh + gap;
                        }

                        if let Some(stats) = &c.stats_line {
                            let (_, fh) = measure_line(hdc, font_small, stats, text_w);
                            draw_line(
                                hdc,
                                font_small,
                                stats_c,
                                stats,
                                line_rect(y, fh),
                            );
                            y += fh + gap;
                        }

                        if let Some(up) = &c.uptime_line {
                            let (_, uh) = measure_line(hdc, font_small, up, text_w);
                            draw_line(
                                hdc,
                                font_small,
                                up_c,
                                up,
                                line_rect(y, uh),
                            );
                        }

                        DeleteObject(font_title as HGDIOBJ);
                        DeleteObject(font_body as HGDIOBJ);
                        DeleteObject(font_small as HGDIOBJ);
                    }
                });

                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, msg, w_param, l_param),
        }
    }

    unsafe fn ensure_window() -> HWND {
        TIP.with(|cell| {
            let mut st = cell.borrow_mut();
            if !st.hwnd.is_null() {
                return st.hwnd;
            }
            let class_name = encode_wide("NetMonFloatingTooltip");
            let wc = WNDCLASSEXW {
                cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_DROPSHADOW,
                lpfn_wnd_proc: wnd_proc as *const () as usize,
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
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                encode_wide("").as_ptr(),
                WS_POPUP,
                0,
                0,
                100,
                50,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            st.hwnd = hwnd;
            hwnd
        })
    }

    pub fn show(
        check: &CheckState,
        windows: UptimeWindows,
        lang: Language,
        is_dark: bool,
        row_rect: Rect,
        window_outer: Rect,
        scale: f32,
    ) {
        unsafe {
            let hwnd = ensure_window();
            if hwnd.is_null() {
                return;
            }

            let content = build_content(check, windows, lang, is_dark);
            let scale = scale.max(1.0);

            // Экранные координаты строки и окна гаджета (в физических пикселях для Win32)
            let win_left = ((window_outer.left()) * scale).round() as i32;
            let win_right = ((window_outer.right()) * scale).round() as i32;
            let row_top = ((window_outer.top() + row_rect.top()) * scale).round() as i32;
            let anchor = (win_left, win_right, row_top, (scale * 100.0).round() as i32);

            let already_up_to_date = TIP.with(|cell| {
                let st = cell.borrow();
                st.visible
                    && st.content.as_ref() == Some(&content)
                    && st.is_dark == is_dark
                    && st.last_anchor == anchor
            });
            if already_up_to_date {
                return;
            }

            let (tip_w, tip_h) = compute_layout(hwnd, &content, scale);

            let monitors = crate::monitor::get_all_monitors();
            let phys_outer = Rect::from_min_max(
                Pos2::new(window_outer.left() * scale, window_outer.top() * scale),
                Pos2::new(window_outer.right() * scale, window_outer.bottom() * scale),
            );
            let work = crate::monitor::best_monitor_for_rect(&monitors, &phys_outer)
                .map(|m| m.work_rect)
                .unwrap_or_else(|| Rect::from_min_max(Pos2::ZERO, Pos2::new(1920.0, 1080.0)));

            let gap = (8.0 * scale).round() as i32;
            let w_left = work.left().round() as i32;
            let w_top = work.top().round() as i32;
            let w_right = work.right().round() as i32;
            let w_bottom = work.bottom().round() as i32;

            // Предпочитаем показывать слева от гаджета (если он у правого края), иначе справа
            let mut x = if win_left - gap - tip_w >= w_left {
                win_left - gap - tip_w
            } else if win_right + gap + tip_w <= w_right {
                win_right + gap
            } else {
                (win_left - gap - tip_w).max(w_left)
            };

            let mut y = row_top - (4.0 * scale).round() as i32;
            x = x.clamp(w_left, (w_right - tip_w).max(w_left));
            y = y.clamp(w_top, (w_bottom - tip_h).max(w_top));

            let new_rect = (x, y, tip_w, tip_h);
            TIP.with(|cell| {
                let mut st = cell.borrow_mut();
                let changed_content = st.content.as_ref() != Some(&content)
                    || (st.scale - scale).abs() > 0.01
                    || st.is_dark != is_dark;
                let changed_pos = st.last_rect != new_rect || !st.visible;

                if changed_content || changed_pos {
                    st.content = Some(content);
                    st.scale = scale;
                    st.is_dark = is_dark;
                    st.last_rect = new_rect;
                    st.last_anchor = anchor;
                    st.visible = true;
                    SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        x,
                        y,
                        tip_w,
                        tip_h,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                    if changed_content {
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    }
                } else {
                    st.last_anchor = anchor;
                }
            });
        }
    }

    pub fn hide() {
        TIP.with(|cell| {
            let mut st = cell.borrow_mut();
            if st.visible && !st.hwnd.is_null() {
                st.visible = false;
                st.content = None;
                st.last_anchor = (0, 0, 0, 0);
                unsafe {
                    ShowWindow(st.hwnd, SW_HIDE);
                }
            }
        });
    }
}

pub fn show(
    check: &CheckState,
    windows: UptimeWindows,
    lang: Language,
    is_dark: bool,
    row_rect: Rect,
    window_outer: Rect,
    scale: f32,
) {
    #[cfg(target_os = "windows")]
    win::show(check, windows, lang, is_dark, row_rect, window_outer, scale);
    #[cfg(not(target_os = "windows"))]
    let _ = (check, windows, lang, is_dark, row_rect, window_outer, scale);
}

pub fn hide() {
    #[cfg(target_os = "windows")]
    win::hide();
}
