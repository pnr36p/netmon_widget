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
