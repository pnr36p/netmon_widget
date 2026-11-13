#![allow(clippy::upper_case_acronyms)]

#[cfg(target_os = "windows")]
pub mod native {
    use crate::i18n::{tr, Language};
    use std::cell::RefCell;
    use std::ffi::c_void;

    type HWND = *mut c_void;
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
    type ATOM = u16;
    #[allow(non_camel_case_types)]
    type UINT_PTR = usize;
    type LPCWSTR = *const u16;

    #[repr(C)]
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

    #[repr(C)]
    struct WNDCLASSEXW {
        cb_size: UINT,
        style: UINT,
        lpfn_wnd_proc: usize,
        cb_cls_extra: i32,
        cb_wnd_extra: i32,
        h_instance: *mut c_void,
        h_icon: *mut c_void,
        h_cursor: *mut c_void,
        hbr_background: HBRUSH,
        lpsz_menu_name: LPCWSTR,
        lpsz_class_name: LPCWSTR,
        h_icon_sm: *mut c_void,
    }

    #[repr(C)]
    struct MENUINFO {
        cb_size: DWORD,
        f_mask: DWORD,
        dw_style: DWORD,
        cy_max: UINT,
        hbr_back: HBRUSH,
        dw_context_help_id: DWORD,
        dw_menu_data: usize,
    }

    #[repr(C)]
    struct MEASUREITEMSTRUCT {
        ctl_type: UINT,
        ctl_id: UINT,
        item_id: UINT,
        item_width: UINT,
        item_height: UINT,
        item_data: usize,
    }

    #[repr(C)]
    struct DRAWITEMSTRUCT {
        ctl_type: UINT,
        ctl_id: UINT,
        item_id: UINT,
        item_action: UINT,
        item_state: UINT,
        hwnd_item: HWND,
        hdc: HDC,
        rc_item: RECT,
        item_data: usize,
    }

    const WS_POPUP: DWORD = 0x8000_0000;
    const WS_EX_TOOLWINDOW: DWORD = 0x0000_0080;

    const WM_MEASUREITEM: UINT = 0x002C;
    const WM_DRAWITEM: UINT = 0x002B;

    const MF_POPUP: UINT = 0x0000_0010;
    const MF_OWNERDRAW: UINT = 0x0000_0100;
    const MF_SEPARATOR: UINT = 0x0000_0800;

    const MIM_BACKGROUND: DWORD = 0x0000_0002;
    const MIM_APPLYTOSUBMENUS: DWORD = 0x8000_0000;

    const ODS_SELECTED: UINT = 0x0001;

    const TPM_LEFTALIGN: UINT = 0x0000;
    const TPM_TOPALIGN: UINT = 0x0000;
    const TPM_RIGHTBUTTON: UINT = 0x0002;
    const TPM_RETURNCMD: UINT = 0x0100;
    const TPM_NONOTIFY: UINT = 0x0080;

    const TRANSPARENT: i32 = 1;
    const DT_LEFT: UINT = 0x0000_0000;
    const DT_VCENTER: UINT = 0x0000_0004;
    const DT_SINGLELINE: UINT = 0x0000_0020;
    const DT_CALCRECT: UINT = 0x0000_0400;
    const DT_NOPREFIX: UINT = 0x0000_0800;

    const LOGPIXELSY: i32 = 90;
    const CLEARTYPE_QUALITY: DWORD = 5;
    const FW_NORMAL: i32 = 400;
    const PS_NULL: i32 = 5;

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
            hwnd_parent: HWND,
            h_menu: HMENU,
            h_instance: *mut c_void,
            lp_param: *mut c_void,
        ) -> HWND;
        fn CreatePopupMenu() -> HMENU;
        fn AppendMenuW(hmenu: HMENU, uflags: UINT, uidnewitem: UINT_PTR, lpnewitem: LPCWSTR) -> BOOL;
        fn SetMenuInfo(hmenu: HMENU, lpmi: *const MENUINFO) -> BOOL;
        fn TrackPopupMenu(
            hmenu: HMENU,
            uflags: UINT,
            x: i32,
            y: i32,
            nreserved: i32,
            hwnd: HWND,
            prcrect: *const c_void,
        ) -> i32;
        fn DestroyMenu(hmenu: HMENU) -> BOOL;
        fn GetCursorPos(lppoint: *mut POINT) -> BOOL;
        fn SetForegroundWindow(hwnd: HWND) -> BOOL;
        fn PostMessageW(hwnd: HWND, msg: UINT, w_param: usize, l_param: isize) -> BOOL;
        fn GetDC(hwnd: HWND) -> HDC;
        fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
        fn FillRect(hdc: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
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
        fn CreateSolidBrush(color: u32) -> HBRUSH;
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
        fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
        fn DeleteObject(ho: HGDIOBJ) -> BOOL;
        fn SetBkMode(hdc: HDC, mode: i32) -> i32;
        fn SetTextColor(hdc: HDC, color: u32) -> u32;
        fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
        fn CreatePen(style: i32, width: i32, color: u32) -> HGDIOBJ;
        fn Polygon(hdc: HDC, apt: *const POINT, cpt: i32) -> BOOL;
        fn ExcludeClipRect(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    }

    fn rgb(r: u8, g: u8, b: u8) -> u32 {
        (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
    }

    fn encode_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    #[derive(Clone)]
    struct MenuEntry {
        label: String,
        checked: bool,
        is_sep: bool,
        has_submenu: bool,
    }

    struct OwnerState {
        hwnd: HWND,
        is_dark: bool,
        scale: f32,
    }

    thread_local! {
        static MENU_OWNER: RefCell<OwnerState> = const {
            RefCell::new(OwnerState {
                hwnd: std::ptr::null_mut(),
                is_dark: true,
                scale: 1.0,
            })
        };
    }

    unsafe fn make_menu_font(scale: f32) -> HFONT {
        let face = encode_wide("Segoe UI");
        let px = -(13.5 * scale).round() as i32;
        CreateFontW(
            px,
            0,
            0,
            0,
            FW_NORMAL,
            0,
            0,
            0,
            1,
            0,
            0,
            CLEARTYPE_QUALITY,
            0,
            face.as_ptr(),
        )
    }

    unsafe extern "system" fn menu_owner_proc(
        hwnd: HWND,
        msg: UINT,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_MEASUREITEM => {
                if l_param == 0 {
                    return 0;
                }
                let mis = &mut *(l_param as *mut MEASUREITEMSTRUCT);
                if mis.item_data == 0 {
                    return 0;
                }
                let entry = &*(mis.item_data as *const MenuEntry);
                let scale = MENU_OWNER.with(|o| o.borrow().scale.max(1.0));
                if entry.is_sep {
                    mis.item_width = (120.0 * scale).round() as UINT;
                    mis.item_height = (8.0 * scale).round() as UINT;
                } else {
                    let hdc = GetDC(hwnd);
                    let font = make_menu_font(scale);
                    let old = SelectObject(hdc, font as HGDIOBJ);
                    let wide: Vec<u16> = entry.label.encode_utf16().collect();
                    let mut rc = RECT {
                        left: 0,
                        top: 0,
                        right: 800,
                        bottom: 40,
                    };
                    DrawTextW(
                        hdc,
                        wide.as_ptr(),
                        wide.len() as i32,
                        &mut rc,
                        DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
                    );
                    SelectObject(hdc, old);
                    DeleteObject(font as HGDIOBJ);
                    ReleaseDC(hwnd, hdc);

                    let left_pad = (26.0 * scale).round() as i32;
                    let right_pad = if entry.has_submenu {
                        (30.0 * scale).round() as i32
                    } else {
                        (18.0 * scale).round() as i32
                    };
                    mis.item_width = ((rc.right - rc.left) + left_pad + right_pad).max(150) as UINT;
                    mis.item_height = ((24.0 * scale).round() as i32).max(20) as UINT;
                }
                1
            }
            WM_DRAWITEM => {
                if l_param == 0 {
                    return 0;
                }
                let dis = &*(l_param as *const DRAWITEMSTRUCT);
                if dis.item_data == 0 {
                    return 0;
                }
                let entry = &*(dis.item_data as *const MenuEntry);
                let (is_dark, scale) = MENU_OWNER.with(|o| {
                    let b = o.borrow();
                    (b.is_dark, b.scale.max(1.0))
                });

                let (bg_col, hover_col, text_col, sep_col, accent_col) = if is_dark {
                    (
                        rgb(0x26, 0x26, 0x28),
                        rgb(0x3A, 0x41, 0x4F),
                        rgb(0xE8, 0xEC, 0xF0),
                        rgb(0x45, 0x4B, 0x56),
                        rgb(0x7A, 0xC5, 0xFF),
                    )
                } else {
                    (
                        rgb(0xFA, 0xFB, 0xFD),
                        rgb(0xDC, 0xE7, 0xF5),
                        rgb(0x1B, 0x20, 0x28),
                        rgb(0xD2, 0xD8, 0xE2),
                        rgb(0x0F, 0x52, 0xA8),
                    )
                };

                let selected = (dis.item_state & ODS_SELECTED) != 0 && !entry.is_sep;
                let bg_br = CreateSolidBrush(if selected { hover_col } else { bg_col });
                FillRect(dis.hdc, &dis.rc_item, bg_br);
                DeleteObject(bg_br as HGDIOBJ);

                if entry.is_sep {
                    let mid_y = (dis.rc_item.top + dis.rc_item.bottom) / 2;
                    let pad = (8.0 * scale).round() as i32;
                    let sep_rc = RECT {
                        left: dis.rc_item.left + pad,
                        top: mid_y,
                        right: dis.rc_item.right - pad,
                        bottom: mid_y + 1,
                    };
                    let sep_br = CreateSolidBrush(sep_col);
                    FillRect(dis.hdc, &sep_rc, sep_br);
                    DeleteObject(sep_br as HGDIOBJ);
                    return 1;
                }

                SetBkMode(dis.hdc, TRANSPARENT);
                let font = make_menu_font(scale);
                let old = SelectObject(dis.hdc, font as HGDIOBJ);

                if entry.checked {
                    SetTextColor(dis.hdc, accent_col);
                    let chk_wide: Vec<u16> = "\u{2713}".encode_utf16().collect();
                    let mut chk_rc = RECT {
                        left: dis.rc_item.left + (8.0 * scale).round() as i32,
                        top: dis.rc_item.top,
                        right: dis.rc_item.left + (24.0 * scale).round() as i32,
                        bottom: dis.rc_item.bottom,
                    };
                    DrawTextW(
                        dis.hdc,
                        chk_wide.as_ptr(),
                        chk_wide.len() as i32,
                        &mut chk_rc,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                    );
                }

                SetTextColor(dis.hdc, text_col);
                let wide: Vec<u16> = entry.label.encode_utf16().collect();
                let mut text_rc = RECT {
                    left: dis.rc_item.left + (26.0 * scale).round() as i32,
                    top: dis.rc_item.top,
                    right: dis.rc_item.right - (14.0 * scale).round() as i32,
                    bottom: dis.rc_item.bottom,
                };
                DrawTextW(
                    dis.hdc,
                    wide.as_ptr(),
                    wide.len() as i32,
                    &mut text_rc,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );

                SelectObject(dis.hdc, old);
                DeleteObject(font as HGDIOBJ);

                if entry.has_submenu {
                    let arrow_col = if is_dark {
                        rgb(0xC8, 0xD2, 0xDE)
                    } else {
                        rgb(0x3A, 0x42, 0x4E)
                    };
                    let mid_y = (dis.rc_item.top + dis.rc_item.bottom) / 2;
                    let tip_x = dis.rc_item.right - (8.0 * scale).round() as i32;
                    let base_x = dis.rc_item.right - (12.0 * scale).round() as i32;
                    let half_h = (4.0 * scale).round().max(3.0) as i32;
                    let pts = [
                        POINT {
                            x: base_x,
                            y: mid_y - half_h,
                        },
                        POINT { x: tip_x, y: mid_y },
                        POINT {
                            x: base_x,
                            y: mid_y + half_h,
                        },
                    ];
                    let brush = CreateSolidBrush(arrow_col);
                    let pen = CreatePen(PS_NULL, 0, 0);
                    let old_br = SelectObject(dis.hdc, brush as HGDIOBJ);
                    let old_pen = SelectObject(dis.hdc, pen as HGDIOBJ);
                    Polygon(dis.hdc, pts.as_ptr(), 3);
                    SelectObject(dis.hdc, old_br);
                    SelectObject(dis.hdc, old_pen);
                    DeleteObject(brush as HGDIOBJ);
                    DeleteObject(pen as HGDIOBJ);

                    // Блокируем область стрелки от стандартной чёрной отрисовки MF_POPUP поверх темы
                    let block_left = dis.rc_item.right - (22.0 * scale).round() as i32;
                    ExcludeClipRect(
                        dis.hdc,
                        block_left,
                        dis.rc_item.top,
                        dis.rc_item.right,
                        dis.rc_item.bottom,
                    );
                }
                1
            }
            _ => DefWindowProcW(hwnd, msg, w_param, l_param),
        }
    }

    unsafe fn ensure_owner_hwnd(is_dark: bool) -> HWND {
        MENU_OWNER.with(|cell| {
            let mut st = cell.borrow_mut();
            st.is_dark = is_dark;
            if st.hwnd.is_null() {
                let class_name = encode_wide("NetMonDarkMenuOwner");
                let wc = WNDCLASSEXW {
                    cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: 0,
                    lpfn_wnd_proc: menu_owner_proc as *const () as usize,
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
                    encode_wide("").as_ptr(),
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
                let hdc = GetDC(hwnd);
                let dpi_y = GetDeviceCaps(hdc, LOGPIXELSY);
                ReleaseDC(hwnd, hdc);
                st.scale = (dpi_y as f32 / 96.0).max(1.0);
                st.hwnd = hwnd;
            }
            st.hwnd
        })
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MenuTarget<'a> {
        Tray,
        EmptyArea,
        Section {
            sec_idx: usize,
            name: &'a str,
        },
        Check {
            sec_idx: usize,
            chk_idx: usize,
            name: &'a str,
            sec_name: &'a str,
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NativeMenuAction {
        EditCheck(usize, usize),
        CloneCheck(usize, usize),
        AddCheck(usize),
        DeleteCheck(usize, usize),
        AddGroup,
        RenameGroup(usize),
        DeleteGroup(usize),
        SetInterval(u64),
        Refresh,
        Settings,
        ToggleAlwaysOnTop,
        ToggleHideTaskbar,
        ToggleMinimize,
        ImportCsv,
        EditConfig,
        RestartToUpdate,
        Close,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct PopupMenuState {
        pub lang: Language,
        pub is_dark: bool,
        pub always_on_top: bool,
        pub minimized: bool,
        pub hide_taskbar: bool,
        pub update_ready: bool,
        pub interval_sec: u64,
    }

    struct MenuBuilder {
        // Box обеспечивает стабильные адреса в памяти для указателей item_data в MF_OWNERDRAW при росте Vec.
        #[allow(clippy::vec_box)]
        entries: Vec<Box<MenuEntry>>,
        action_ids: Vec<NativeMenuAction>,
    }

    impl MenuBuilder {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
                action_ids: Vec::new(),
            }
        }

        unsafe fn add_item(
            &mut self,
            menu: HMENU,
            label: &str,
            checked: bool,
            act: NativeMenuAction,
        ) {
            self.action_ids.push(act);
            let id = self.action_ids.len();
            let mut boxed = Box::new(MenuEntry {
                label: label.to_string(),
                checked,
                is_sep: false,
                has_submenu: false,
            });
            let ptr = (&mut *boxed as *mut MenuEntry) as *const u16;
            self.entries.push(boxed);
            AppendMenuW(menu, MF_OWNERDRAW, id, ptr);
        }

        unsafe fn add_sep(&mut self, menu: HMENU) {
            let mut boxed = Box::new(MenuEntry {
                label: String::new(),
                checked: false,
                is_sep: true,
                has_submenu: false,
            });
            let ptr = (&mut *boxed as *mut MenuEntry) as *const u16;
            self.entries.push(boxed);
            AppendMenuW(menu, MF_OWNERDRAW | MF_SEPARATOR, 0, ptr);
        }

        unsafe fn add_popup(&mut self, parent: HMENU, sub: HMENU, label: String) {
            let mut boxed = Box::new(MenuEntry {
                label,
                checked: false,
                is_sep: false,
                has_submenu: true,
            });
            let ptr = (&mut *boxed as *mut MenuEntry) as *const u16;
            self.entries.push(boxed);
            AppendMenuW(parent, MF_OWNERDRAW | MF_POPUP, sub as usize, ptr);
        }
    }

    pub fn show_popup(
        opts: PopupMenuState,
        target: MenuTarget<'_>,
    ) -> Option<NativeMenuAction> {
        let lang = opts.lang;
        let is_dark = opts.is_dark;
        let always_on_top = opts.always_on_top;
        let minimized = opts.minimized;
        let hide_taskbar = opts.hide_taskbar;
        let update_ready = opts.update_ready;
        let interval_sec = opts.interval_sec;
        let s = tr(lang);
        unsafe {
            let owner_hwnd = ensure_owner_hwnd(is_dark);
            if owner_hwnd.is_null() {
                return None;
            }

            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            let cursor_pos = eframe::egui::Pos2::new(pt.x as f32, pt.y as f32);
            let monitors = crate::monitor::get_all_monitors();
            if let Some(mon) = monitors
                .iter()
                .find(|m| m.monitor_rect.contains(cursor_pos))
                .or_else(|| crate::monitor::primary_or_first(&monitors))
            {
                MENU_OWNER.with(|o| o.borrow_mut().scale = mon.scale.max(1.0));
            }

            let hmenu = CreatePopupMenu();
            if hmenu.is_null() {
                return None;
            }

            let mut mb = MenuBuilder::new();

            match target {
                MenuTarget::Check {
                    sec_idx,
                    chk_idx,
                    name,
                    sec_name,
                } => {
                    let edit_lbl = s.menu_edit_host.replace("{}", name);
                    let clone_lbl = s.menu_clone_host.replace("{}", name);
                    let del_lbl = s.menu_delete_host.replace("{}", name);
                    let ren_grp = s.menu_rename_group.replace("{}", sec_name);
                    let del_grp = s.menu_delete_group.replace("{}", sec_name);
                    mb.add_item(hmenu, &edit_lbl, false, NativeMenuAction::EditCheck(sec_idx, chk_idx));
                    mb.add_item(hmenu, &clone_lbl, false, NativeMenuAction::CloneCheck(sec_idx, chk_idx));
                    mb.add_item(hmenu, s.menu_add_host, false, NativeMenuAction::AddCheck(sec_idx));
                    mb.add_item(hmenu, &del_lbl, false, NativeMenuAction::DeleteCheck(sec_idx, chk_idx));
                    mb.add_sep(hmenu);
                    mb.add_item(hmenu, s.menu_add_group, false, NativeMenuAction::AddGroup);
                    mb.add_item(hmenu, &ren_grp, false, NativeMenuAction::RenameGroup(sec_idx));
                    mb.add_item(hmenu, &del_grp, false, NativeMenuAction::DeleteGroup(sec_idx));
                    mb.add_sep(hmenu);
                }
                MenuTarget::Section { sec_idx, name } => {
                    let add_in = s.menu_add_host_in_group.replace("{}", name);
                    let ren_grp = s.menu_rename_group.replace("{}", name);
                    let del_grp = s.menu_delete_group.replace("{}", name);
                    mb.add_item(hmenu, &add_in, false, NativeMenuAction::AddCheck(sec_idx));
                    mb.add_item(hmenu, s.menu_add_group, false, NativeMenuAction::AddGroup);
                    mb.add_item(hmenu, &ren_grp, false, NativeMenuAction::RenameGroup(sec_idx));
                    mb.add_item(hmenu, &del_grp, false, NativeMenuAction::DeleteGroup(sec_idx));
                    mb.add_sep(hmenu);
                }
                MenuTarget::EmptyArea => {
                    mb.add_item(hmenu, s.menu_add_host, false, NativeMenuAction::AddCheck(0));
                    mb.add_item(hmenu, s.menu_add_group, false, NativeMenuAction::AddGroup);
                    mb.add_sep(hmenu);
                }
                MenuTarget::Tray => {
                    mb.add_item(
                        hmenu,
                        if minimized {
                            s.menu_restore_from_tray
                        } else {
                            s.menu_minimize_to_tray
                        },
                        false,
                        NativeMenuAction::ToggleMinimize,
                    );
                    mb.add_sep(hmenu);
                }
            }

            mb.add_item(hmenu, s.menu_refresh, false, NativeMenuAction::Refresh);

            // Подменю общего интервала опроса
            let sub_interval = CreatePopupMenu();
            if !sub_interval.is_null() {
                let mut intervals = vec![1u64, 3, 5, 10, 15, 30, 60];
                if !intervals.contains(&interval_sec) && interval_sec > 0 {
                    intervals.push(interval_sec);
                    intervals.sort_unstable();
                }
                for sec in intervals {
                    let lbl = format!("{} {}", sec, s.settings_sec_unit);
                    mb.add_item(
                        sub_interval,
                        &lbl,
                        sec == interval_sec,
                        NativeMenuAction::SetInterval(sec),
                    );
                }
                let sub_label = format!("{} ({} {})", s.menu_interval, interval_sec, s.settings_sec_unit);
                mb.add_popup(hmenu, sub_interval, sub_label);
            }

            mb.add_item(hmenu, s.menu_settings, false, NativeMenuAction::Settings);
            mb.add_item(
                hmenu,
                s.menu_always_on_top,
                always_on_top,
                NativeMenuAction::ToggleAlwaysOnTop,
            );
            mb.add_item(
                hmenu,
                s.menu_hide_taskbar,
                hide_taskbar,
                NativeMenuAction::ToggleHideTaskbar,
            );
            mb.add_sep(hmenu);
            mb.add_item(hmenu, s.menu_import_csv, false, NativeMenuAction::ImportCsv);
            mb.add_item(hmenu, s.menu_edit_config, false, NativeMenuAction::EditConfig);
            mb.add_sep(hmenu);
            if update_ready {
                mb.add_item(
                    hmenu,
                    s.menu_restart_update,
                    false,
                    NativeMenuAction::RestartToUpdate,
                );
            }
            if !matches!(target, MenuTarget::Tray) {
                mb.add_item(
                    hmenu,
                    s.menu_minimize_to_tray,
                    false,
                    NativeMenuAction::ToggleMinimize,
                );
            }
            mb.add_item(hmenu, s.menu_close_app, false, NativeMenuAction::Close);

            let bg_brush = CreateSolidBrush(if is_dark {
                rgb(0x26, 0x26, 0x28)
            } else {
                rgb(0xFA, 0xFB, 0xFD)
            });
            let mi = MENUINFO {
                cb_size: std::mem::size_of::<MENUINFO>() as DWORD,
                f_mask: MIM_BACKGROUND | MIM_APPLYTOSUBMENUS,
                dw_style: 0,
                cy_max: 0,
                hbr_back: bg_brush,
                dw_context_help_id: 0,
                dw_menu_data: 0,
            };
            SetMenuInfo(hmenu, &mi);

            SetForegroundWindow(owner_hwnd);

            let cmd = TrackPopupMenu(
                hmenu,
                TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
                pt.x,
                pt.y,
                0,
                owner_hwnd,
                std::ptr::null(),
            );

            PostMessageW(owner_hwnd, 0, 0, 0);
            DestroyMenu(hmenu);
            DeleteObject(bg_brush as HGDIOBJ);
            let result = if cmd > 0 {
                mb.action_ids.get((cmd as usize).saturating_sub(1)).copied()
            } else {
                None
            };
            drop(mb);
            result
        }
    }
}
