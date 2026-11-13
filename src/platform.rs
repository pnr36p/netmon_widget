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
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(
            lp_enum_func: unsafe extern "system" fn(isize, isize) -> i32,
            l_param: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, lpdw_process_id: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn GetWindowTextW(hwnd: isize, lp_string: *mut u16, n_max_count: i32) -> i32;
    }

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
            let mut hwnd: isize = 0;
            EnumWindows(find_main_hwnd_proc, (&mut hwnd as *mut isize) as isize);
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
