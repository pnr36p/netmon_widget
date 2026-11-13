#![allow(clippy::upper_case_acronyms)]

use std::path::PathBuf;

/// Стабильный 32-битный FNV-1a хеш для уникального имени ключа автозагрузки по пути к exe.
fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for &b in data {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

pub fn autostart_value_name_for_exe(exe_path: &std::path::Path) -> String {
    let normalized = exe_path.to_string_lossy().to_lowercase();
    format!("NetmonWidget_{:08x}", fnv1a_32(normalized.as_bytes()))
}

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;
    use std::path::Path;

    type HKEY = *mut c_void;
    type LPCWSTR = *const u16;
    type LPBYTE = *mut u8;
    type LPDWORD = *mut u32;
    type DWORD = u32;
    type LSTATUS = i32;

    const HKEY_CURRENT_USER: HKEY = 0x8000_0001u32 as usize as HKEY;
    const KEY_READ: DWORD = 0x20019;
    const KEY_SET_VALUE: DWORD = 0x0002;
    const REG_SZ: DWORD = 1;
    const REG_DWORD: DWORD = 4;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hkey: HKEY,
            lp_sub_key: LPCWSTR,
            ul_options: DWORD,
            sam_desired: DWORD,
            phk_result: *mut HKEY,
        ) -> LSTATUS;
        fn RegQueryValueExW(
            hkey: HKEY,
            lp_value_name: LPCWSTR,
            lp_reserved: LPDWORD,
            lp_type: LPDWORD,
            lp_data: LPBYTE,
            lpcb_data: LPDWORD,
        ) -> LSTATUS;
        fn RegSetValueExW(
            hkey: HKEY,
            lp_value_name: LPCWSTR,
            reserved: DWORD,
            dw_type: DWORD,
            lp_data: *const u8,
            cb_data: DWORD,
        ) -> LSTATUS;
        fn RegDeleteValueW(hkey: HKEY, lp_value_name: LPCWSTR) -> LSTATUS;
        fn RegCloseKey(hkey: HKEY) -> LSTATUS;
    }

    fn encode_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn is_system_dark_theme() -> bool {
        unsafe {
            let subkey = encode_wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
            let val_name = encode_wide("AppsUseLightTheme");
            let mut hkey: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
                return true;
            }
            let mut val: u32 = 0;
            let mut val_type: DWORD = 0;
            let mut size: DWORD = std::mem::size_of::<u32>() as DWORD;
            let status = RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                &mut val as *mut u32 as LPBYTE,
                &mut size,
            );
            RegCloseKey(hkey);
            if status == 0 && val_type == REG_DWORD {
                val == 0
            } else {
                true
            }
        }
    }

    pub fn is_autostart_enabled(exe_path: &Path) -> bool {
        let name = super::autostart_value_name_for_exe(exe_path);
        unsafe {
            let subkey = encode_wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
            let val_name = encode_wide(&name);
            let mut hkey: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
                return false;
            }
            let mut val_type: DWORD = 0;
            let mut size: DWORD = 0;
            let status = RegQueryValueExW(
                hkey,
                val_name.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                std::ptr::null_mut(),
                &mut size,
            );
            RegCloseKey(hkey);
            status == 0 && val_type == REG_SZ && size > 2
        }
    }

    pub fn set_autostart(exe_path: &Path, enabled: bool) -> Result<(), String> {
        let name = super::autostart_value_name_for_exe(exe_path);
        unsafe {
            let subkey = encode_wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
            let val_name = encode_wide(&name);
            let mut hkey: HKEY = std::ptr::null_mut();
            let st = RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_SET_VALUE, &mut hkey);
            if st != 0 {
                return Err(format!("RegOpenKeyExW error {}", st));
            }
            let res = if enabled {
                let cmd = format!("\"{}\"", exe_path.to_string_lossy());
                let wide = encode_wide(&cmd);
                let byte_len = (wide.len() * 2) as DWORD;
                let s = RegSetValueExW(
                    hkey,
                    val_name.as_ptr(),
                    0,
                    REG_SZ,
                    wide.as_ptr() as *const u8,
                    byte_len,
                );
                if s == 0 {
                    Ok(())
                } else {
                    Err(format!("RegSetValueExW error {}", s))
                }
            } else {
                let s = RegDeleteValueW(hkey, val_name.as_ptr());
                // ERROR_FILE_NOT_FOUND = 2 тоже считаем успехом при отключении
                if s == 0 || s == 2 {
                    Ok(())
                } else {
                    Err(format!("RegDeleteValueW error {}", s))
                }
            };
            RegCloseKey(hkey);
            res
        }
    }

    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct ITaskbarListVtbl {
        query_interface: unsafe extern "system" fn(*mut std::ffi::c_void, *const Guid, *mut *mut std::ffi::c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
        release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
        hr_init: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
        add_tab: unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32,
        delete_tab: unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32,
    }

    #[repr(C)]
    struct ITaskbarList {
        vtbl: *const ITaskbarListVtbl,
    }

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(pv_reserved: *mut std::ffi::c_void, dw_co_init: u32) -> i32;
        fn CoUninitialize();
        fn CoCreateInstance(
            rclsid: *const Guid,
            p_unk_outer: *mut std::ffi::c_void,
            dw_cls_context: u32,
            riid: *const Guid,
            ppv: *mut *mut std::ffi::c_void,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcessId() -> u32;
        fn GetModuleHandleA(lp_module_name: *const std::ffi::c_char) -> *mut std::ffi::c_void;
        fn GetProcAddress(
            h_module: *mut std::ffi::c_void,
            lp_proc_name: *const std::ffi::c_char,
        ) -> *const std::ffi::c_void;
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct POINT {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(
            lp_enum_func: unsafe extern "system" fn(isize, isize) -> i32,
            l_param: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, lpdw_process_id: *mut u32) -> u32;
        fn IsWindow(hwnd: isize) -> i32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn GetWindowTextW(hwnd: isize, lp_string: *mut u16, n_max_count: i32) -> i32;
        fn GetWindowRect(hwnd: isize, lp_rect: *mut RECT) -> i32;
        fn SetWindowPos(
            hwnd: *mut std::ffi::c_void,
            hwnd_insert_after: *mut std::ffi::c_void,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
        fn GetCursorPos(lp_point: *mut POINT) -> i32;
        fn MonitorFromRect(lprc: *const RECT, dw_flags: u32) -> *mut std::ffi::c_void;
        fn GetMonitorInfoW(
            hmonitor: *mut std::ffi::c_void,
            lpmi: *mut crate::monitor::MONITORINFOEXW,
        ) -> i32;
        fn SetWindowLongPtrW(hwnd: isize, n_index: i32, dw_new_long: isize) -> isize;
        fn CallWindowProcW(
            lp_prev_wnd_func: isize,
            hwnd: isize,
            msg: u32,
            w_param: usize,
            l_param: isize,
        ) -> isize;
    }

    use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};

    static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);
    static ORIG_WNDPROC: AtomicIsize = AtomicIsize::new(0);
    static EDGE_SNAPPING_ENABLED: AtomicBool = AtomicBool::new(true);
    static SIZEMOVE_FINISHED: AtomicBool = AtomicBool::new(false);
    static DRAG_ANCHOR_VALID: AtomicBool = AtomicBool::new(false);
    static DRAG_CURSOR_X: AtomicI32 = AtomicI32::new(0);
    static DRAG_CURSOR_Y: AtomicI32 = AtomicI32::new(0);
    static DRAG_RECT_LEFT: AtomicI32 = AtomicI32::new(0);
    static DRAG_RECT_TOP: AtomicI32 = AtomicI32::new(0);
    static DRAG_RECT_W: AtomicI32 = AtomicI32::new(1);
    static DRAG_RECT_H: AtomicI32 = AtomicI32::new(1);

    const WM_MOVING: u32 = 0x0216;
    const WM_ENTERSIZEMOVE: u32 = 0x0231;
    const WM_EXITSIZEMOVE: u32 = 0x0232;
    const GWLP_WNDPROC: i32 = -4;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;

    unsafe extern "system" fn find_main_hwnd_proc(hwnd: isize, lparam: isize) -> i32 {
        unsafe {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid != GetCurrentProcessId() || IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            let mut buf = [0u16; 128];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                if title == "netmon_widget" {
                    let out = lparam as *mut isize;
                    *out = hwnd;
                    return 0;
                }
            }
            1
        }
    }

    pub fn set_skip_taskbar(skip: bool) {
        const COINIT_APARTMENTTHREADED: u32 = 0x2;
        const CLSCTX_INPROC_SERVER: u32 = 0x1;
        const RPC_E_CHANGED_MODE: i32 = -2147417850; // 0x80010106
        const CLSID_TASKBAR_LIST: Guid = Guid {
            data1: 0x56FDF344,
            data2: 0xFD6D,
            data3: 0x11D0,
            data4: [0x95, 0x8A, 0x00, 0x60, 0x97, 0xC9, 0xA0, 0x90],
        };
        const IID_ITASKBAR_LIST: Guid = Guid {
            data1: 0x56FDF342,
            data2: 0xFD6D,
            data3: 0x11D0,
            data4: [0x95, 0x8A, 0x00, 0x60, 0x97, 0xC9, 0xA0, 0x90],
        };

        unsafe {
            let hwnd = find_main_hwnd();
            if hwnd == 0 {
                return;
            }

            let hr_init_com = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
            let need_uninit = hr_init_com == 0 || hr_init_com == 1;
            if hr_init_com < 0 && hr_init_com != RPC_E_CHANGED_MODE {
                return;
            }

            let mut obj: *mut std::ffi::c_void = std::ptr::null_mut();
            let hr = CoCreateInstance(
                &CLSID_TASKBAR_LIST,
                std::ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_ITASKBAR_LIST,
                &mut obj,
            );
            if hr >= 0 && !obj.is_null() {
                let tbl = obj as *mut ITaskbarList;
                let vtbl = &*(*tbl).vtbl;
                if (vtbl.hr_init)(obj) >= 0 {
                    if skip {
                        (vtbl.delete_tab)(obj, hwnd);
                    } else {
                        (vtbl.add_tab)(obj, hwnd);
                    }
                }
                (vtbl.release)(obj);
            }

            if need_uninit {
                CoUninitialize();
            }
        }
    }

    fn find_main_hwnd() -> isize {
        let cached = MAIN_HWND.load(Ordering::Relaxed);
        if cached != 0 && unsafe { IsWindow(cached) != 0 } {
            return cached;
        }
        let mut hwnd: isize = 0;
        unsafe {
            EnumWindows(find_main_hwnd_proc, (&mut hwnd as *mut isize) as isize);
        }
        if hwnd != 0 {
            MAIN_HWND.store(hwnd, Ordering::Relaxed);
        }
        hwnd
    }

    fn window_dpi_scale(hwnd: isize) -> f32 {
        type GetDpiForWindowFn = unsafe extern "system" fn(isize) -> u32;
        static GET_DPI_FOR_WINDOW: std::sync::OnceLock<Option<GetDpiForWindowFn>> =
            std::sync::OnceLock::new();

        let func_opt = GET_DPI_FOR_WINDOW.get_or_init(|| unsafe {
            let hmod = GetModuleHandleA(c"user32.dll".as_ptr());
            if hmod.is_null() {
                return None;
            }
            let proc = GetProcAddress(hmod, c"GetDpiForWindow".as_ptr());
            if proc.is_null() {
                return None;
            }
            Some(std::mem::transmute::<*const std::ffi::c_void, GetDpiForWindowFn>(proc))
        });

        if let Some(func) = func_opt {
            unsafe {
                let dpi = func(hwnd);
                if dpi >= 48 {
                    return (dpi as f32 / 96.0).clamp(0.75, 4.0);
                }
            }
        }
        1.0
    }

    unsafe fn work_rect_for_rect(r: &RECT) -> Option<RECT> {
        let hmon = MonitorFromRect(r, MONITOR_DEFAULTTONEAREST);
        if hmon.is_null() {
            return None;
        }
        let mut mi: crate::monitor::MONITORINFOEXW = std::mem::zeroed();
        mi.cb_size = std::mem::size_of::<crate::monitor::MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, &mut mi) != 0 {
            Some(RECT {
                left: mi.rc_work.left,
                top: mi.rc_work.top,
                right: mi.rc_work.right,
                bottom: mi.rc_work.bottom,
            })
        } else {
            None
        }
    }

    unsafe extern "system" fn subclass_wndproc(
        hwnd: isize,
        msg: u32,
        w_param: usize,
        l_param: isize,
    ) -> isize {
        if msg == WM_ENTERSIZEMOVE {
            unsafe {
                let mut pt: POINT = std::mem::zeroed();
                let mut wr: RECT = std::mem::zeroed();
                if GetCursorPos(&mut pt) != 0 && GetWindowRect(hwnd, &mut wr) != 0 {
                    DRAG_CURSOR_X.store(pt.x, Ordering::Relaxed);
                    DRAG_CURSOR_Y.store(pt.y, Ordering::Relaxed);
                    DRAG_RECT_LEFT.store(wr.left, Ordering::Relaxed);
                    DRAG_RECT_TOP.store(wr.top, Ordering::Relaxed);
                    DRAG_RECT_W.store((wr.right - wr.left).max(1), Ordering::Relaxed);
                    DRAG_RECT_H.store((wr.bottom - wr.top).max(1), Ordering::Relaxed);
                    DRAG_ANCHOR_VALID.store(true, Ordering::Relaxed);
                }
            }
        } else if msg == WM_MOVING && l_param != 0 && EDGE_SNAPPING_ENABLED.load(Ordering::Relaxed)
        {
            unsafe {
                let r = &mut *(l_param as *mut RECT);
                let w = r.right - r.left;
                let h = r.bottom - r.top;
                // Не прижимаем служебное свёрнутое окно (1x1 за экраном)
                if w > 10 && h > 10 {
                    let mut pt: POINT = std::mem::zeroed();
                    if GetCursorPos(&mut pt) != 0 {
                        if !DRAG_ANCHOR_VALID.load(Ordering::Relaxed) {
                            DRAG_CURSOR_X.store(pt.x, Ordering::Relaxed);
                            DRAG_CURSOR_Y.store(pt.y, Ordering::Relaxed);
                            DRAG_RECT_LEFT.store(r.left, Ordering::Relaxed);
                            DRAG_RECT_TOP.store(r.top, Ordering::Relaxed);
                            DRAG_RECT_W.store(w.max(1), Ordering::Relaxed);
                            DRAG_RECT_H.store(h.max(1), Ordering::Relaxed);
                            DRAG_ANCHOR_VALID.store(true, Ordering::Relaxed);
                        }

                        // Считаем «истинную» позицию от стартовой точки курсора, чтобы медленное отлипание
                        // не терялось из-за инкрементальной логики SC_MOVE
                        let start_cx = DRAG_CURSOR_X.load(Ordering::Relaxed);
                        let start_cy = DRAG_CURSOR_Y.load(Ordering::Relaxed);
                        let start_l = DRAG_RECT_LEFT.load(Ordering::Relaxed);
                        let start_t = DRAG_RECT_TOP.load(Ordering::Relaxed);
                        let start_w = DRAG_RECT_W.load(Ordering::Relaxed).max(1);
                        let start_h = DRAG_RECT_H.load(Ordering::Relaxed).max(1);

                        let grab_x = start_cx - start_l;
                        let grab_y = start_cy - start_t;
                        let adj_grab_x = ((grab_x as f32) * (w as f32) / (start_w as f32)).round() as i32;
                        let adj_grab_y = ((grab_y as f32) * (h as f32) / (start_h as f32)).round() as i32;

                        let true_left = pt.x - adj_grab_x;
                        let true_top = pt.y - adj_grab_y;
                        let true_rect = RECT {
                            left: true_left,
                            top: true_top,
                            right: true_left + w,
                            bottom: true_top + h,
                        };

                        let mut out_left = true_left;
                        let mut out_top = true_top;
                        if let Some(work) = work_rect_for_rect(&true_rect) {
                            let scale = window_dpi_scale(hwnd);
                            let snap_px = (crate::monitor::SNAP_THRESHOLD * scale)
                                .round()
                                .clamp(5.0, 12.0) as i32;

                            let dl = (true_left - work.left).abs();
                            let dr = (work.right - (true_left + w)).abs();
                            if dl <= dr && dl <= snap_px {
                                out_left = work.left;
                            } else if dr <= snap_px {
                                out_left = work.right - w;
                            }

                            let dt = (true_top - work.top).abs();
                            let db = (work.bottom - (true_top + h)).abs();
                            if dt <= db && dt <= snap_px {
                                out_top = work.top;
                            } else if db <= snap_px {
                                out_top = work.bottom - h;
                            }
                        }

                        r.left = out_left;
                        r.top = out_top;
                        r.right = out_left + w;
                        r.bottom = out_top + h;
                    }
                }
            }
        } else if msg == WM_EXITSIZEMOVE {
            DRAG_ANCHOR_VALID.store(false, Ordering::Relaxed);
            SIZEMOVE_FINISHED.store(true, Ordering::Release);
        }

        let orig = ORIG_WNDPROC.load(Ordering::Relaxed);
        if orig != 0 {
            unsafe { CallWindowProcW(orig, hwnd, msg, w_param, l_param) }
        } else {
            0
        }
    }

    pub fn ensure_window_hook() {
        let hwnd = find_main_hwnd();
        if hwnd == 0 || ORIG_WNDPROC.load(Ordering::Relaxed) != 0 {
            return;
        }
        unsafe {
            let prev = SetWindowLongPtrW(
                hwnd,
                GWLP_WNDPROC,
                subclass_wndproc as *const () as isize,
            );
            if prev != 0 {
                ORIG_WNDPROC.store(prev, Ordering::Relaxed);
            }
        }
    }

    pub fn set_edge_snapping(enabled: bool) {
        EDGE_SNAPPING_ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub fn take_size_move_finished() -> bool {
        SIZEMOVE_FINISHED.swap(false, Ordering::AcqRel)
    }

    pub fn get_window_rect_physical() -> Option<eframe::egui::Rect> {
        let hwnd = find_main_hwnd();
        if hwnd == 0 {
            return None;
        }
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut r) != 0 {
                Some(eframe::egui::Rect::from_min_max(
                    eframe::egui::Pos2::new(r.left as f32, r.top as f32),
                    eframe::egui::Pos2::new(r.right as f32, r.bottom as f32),
                ))
            } else {
                None
            }
        }
    }

    pub fn set_window_pos_physical(x: i32, y: i32) {
        let hwnd = find_main_hwnd();
        if hwnd == 0 {
            return;
        }
        unsafe {
            SetWindowPos(
                hwnd as *mut std::ffi::c_void,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn get_window_dpi_scale() -> f32 {
        let hwnd = find_main_hwnd();
        if hwnd == 0 {
            1.0
        } else {
            window_dpi_scale(hwnd)
        }
    }

    pub fn begin_window_resize(edge: super::ResizeEdge) -> Option<super::WindowResizeSession> {
        let hwnd = find_main_hwnd();
        if hwnd == 0 {
            return None;
        }
        unsafe {
            let mut pt: POINT = std::mem::zeroed();
            let mut r: RECT = std::mem::zeroed();
            if GetCursorPos(&mut pt) == 0 || GetWindowRect(hwnd, &mut r) == 0 {
                return None;
            }
            let rect_arr = [r.left, r.top, r.right, r.bottom];
            Some(super::WindowResizeSession {
                edge,
                start_cursor: [pt.x, pt.y],
                start_rect: rect_arr,
                scale: window_dpi_scale(hwnd),
                last_rect: rect_arr,
            })
        }
    }

    pub fn apply_window_resize(session: &mut super::WindowResizeSession, snap_enabled: bool) {
        use super::ResizeEdge;
        let hwnd = find_main_hwnd();
        if hwnd == 0 {
            return;
        }
        unsafe {
            let mut pt: POINT = std::mem::zeroed();
            if GetCursorPos(&mut pt) == 0 {
                return;
            }
            let dx = pt.x - session.start_cursor[0];
            let dy = pt.y - session.start_cursor[1];

            let min_w = (110.0 * session.scale).round() as i32;
            let max_w = (1000.0 * session.scale).round() as i32;
            let min_h = (60.0 * session.scale).round() as i32;
            let max_h = (2000.0 * session.scale).round() as i32;

            let [sl, st, sr, sb] = session.start_rect;
            let mut l = sl;
            let mut t = st;
            let mut r = sr;
            let mut b = sb;

            let moves_left = matches!(
                session.edge,
                ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft
            );
            let moves_right = matches!(
                session.edge,
                ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight
            );
            let moves_top = matches!(
                session.edge,
                ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight
            );
            let moves_bottom = matches!(
                session.edge,
                ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight
            );

            if moves_left {
                l = (sl + dx).clamp(sr - max_w, sr - min_w);
            }
            if moves_right {
                r = (sr + dx).clamp(sl + min_w, sl + max_w);
            }
            if moves_top {
                t = (st + dy).clamp(sb - max_h, sb - min_h);
            }
            if moves_bottom {
                b = (sb + dy).clamp(st + min_h, st + max_h);
            }

            if snap_enabled {
                let probe = RECT {
                    left: l,
                    top: t,
                    right: r,
                    bottom: b,
                };
                if let Some(work) = work_rect_for_rect(&probe) {
                    let snap_px = (crate::monitor::SNAP_THRESHOLD * session.scale).round() as i32;
                    if moves_left && (l - work.left).abs() <= snap_px {
                        let cand_w = r - work.left;
                        if cand_w >= min_w && cand_w <= max_w {
                            l = work.left;
                        }
                    }
                    if moves_right && (work.right - r).abs() <= snap_px {
                        let cand_w = work.right - l;
                        if cand_w >= min_w && cand_w <= max_w {
                            r = work.right;
                        }
                    }
                    if moves_top && (t - work.top).abs() <= snap_px {
                        let cand_h = b - work.top;
                        if cand_h >= min_h && cand_h <= max_h {
                            t = work.top;
                        }
                    }
                    if moves_bottom && (work.bottom - b).abs() <= snap_px {
                        let cand_h = work.bottom - t;
                        if cand_h >= min_h && cand_h <= max_h {
                            b = work.bottom;
                        }
                    }
                }
            }

            let next = [l, t, r, b];
            if next != session.last_rect {
                session.last_rect = next;
                SetWindowPos(
                    hwnd as *mut std::ffi::c_void,
                    std::ptr::null_mut(),
                    l,
                    t,
                    r - l,
                    b - t,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    pub fn cursor_icon(self) -> eframe::egui::CursorIcon {
        match self {
            ResizeEdge::Left | ResizeEdge::Right => eframe::egui::CursorIcon::ResizeHorizontal,
            ResizeEdge::Top | ResizeEdge::Bottom => eframe::egui::CursorIcon::ResizeVertical,
            ResizeEdge::TopLeft | ResizeEdge::BottomRight => {
                eframe::egui::CursorIcon::ResizeNwSe
            }
            ResizeEdge::TopRight | ResizeEdge::BottomLeft => {
                eframe::egui::CursorIcon::ResizeNeSw
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WindowResizeSession {
    pub edge: ResizeEdge,
    pub start_cursor: [i32; 2],
    pub start_rect: [i32; 4],
    pub scale: f32,
    pub last_rect: [i32; 4],
}

pub fn current_exe_path() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("netmon_widget.exe"))
}

pub fn is_system_dark_theme() -> bool {
    #[cfg(target_os = "windows")]
    {
        win::is_system_dark_theme()
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

pub fn is_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        win::is_autostart_enabled(&current_exe_path())
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        win::set_autostart(&current_exe_path(), enabled)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Ok(())
    }
}

pub fn set_skip_taskbar(skip: bool) {
    #[cfg(target_os = "windows")]
    {
        win::set_skip_taskbar(skip);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = skip;
    }
}

pub fn ensure_window_hook() {
    #[cfg(target_os = "windows")]
    win::ensure_window_hook();
}

pub fn set_edge_snapping(enabled: bool) {
    #[cfg(target_os = "windows")]
    win::set_edge_snapping(enabled);
    #[cfg(not(target_os = "windows"))]
    let _ = enabled;
}

pub fn take_size_move_finished() -> bool {
    #[cfg(target_os = "windows")]
    {
        win::take_size_move_finished()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn get_window_rect_physical() -> Option<eframe::egui::Rect> {
    #[cfg(target_os = "windows")]
    {
        win::get_window_rect_physical()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn set_window_pos_physical(x: i32, y: i32) {
    #[cfg(target_os = "windows")]
    win::set_window_pos_physical(x, y);
    #[cfg(not(target_os = "windows"))]
    let _ = (x, y);
}

pub fn get_window_dpi_scale() -> f32 {
    #[cfg(target_os = "windows")]
    {
        win::get_window_dpi_scale()
    }
    #[cfg(not(target_os = "windows"))]
    {
        1.0
    }
}

pub fn begin_window_resize(edge: ResizeEdge) -> Option<WindowResizeSession> {
    #[cfg(target_os = "windows")]
    {
        win::begin_window_resize(edge)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = edge;
        None
    }
}

pub fn apply_window_resize(session: &mut WindowResizeSession, snap_enabled: bool) {
    #[cfg(target_os = "windows")]
    win::apply_window_resize(session, snap_enabled);
    #[cfg(not(target_os = "windows"))]
    let _ = (session, snap_enabled);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_folders_produce_distinct_autostart_names() {
        let a = std::path::Path::new(r"C:\Apps\Work\netmon_widget.exe");
        let b = std::path::Path::new(r"C:\Apps\Home\netmon_widget.exe");
        assert_ne!(autostart_value_name_for_exe(a), autostart_value_name_for_exe(b));
        assert_eq!(
            autostart_value_name_for_exe(a),
            autostart_value_name_for_exe(std::path::Path::new(r"c:\apps\work\netmon_widget.exe"))
        );
    }
}
