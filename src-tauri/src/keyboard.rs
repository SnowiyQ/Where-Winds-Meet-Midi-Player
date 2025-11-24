use enigo::{Enigo, Key, Keyboard, Settings, Direction};
use std::sync::Mutex;

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows,
    GetForegroundWindow,
    GetWindowTextW,
    SetForegroundWindow,
    ShowWindow,
    SW_RESTORE,
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};

#[cfg(target_os = "windows")]
const TARGET_WINDOW_KEYWORDS: [&str; 4] =
    ["where winds meet", "wwm", "wwm.exe", "wwm overlay"];

#[cfg(target_os = "windows")]
struct EnumData {
    target: Option<HWND>,
}

#[cfg(target_os = "windows")]
fn matches_target_window(hwnd: HWND) -> bool {
    let mut title = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut title) };
    if len <= 0 {
        return false;
    }
    let title_string = String::from_utf16_lossy(&title[..len as usize]).to_lowercase();
    TARGET_WINDOW_KEYWORDS
        .iter()
        .any(|keyword| title_string.contains(keyword))
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut EnumData);
    if matches_target_window(hwnd) {
        data.target = Some(hwnd);
        return BOOL(0);
    }
    BOOL(1)
}

lazy_static::lazy_static! {
    static ref ENIGO: Mutex<Enigo> = Mutex::new(
        Enigo::new(&Settings::default()).expect("Failed to initialize Enigo")
    );
}

pub fn key_down(key: &str) {
    let mut enigo = ENIGO.lock().unwrap();

    if let Some(k) = string_to_key(key) {
        let _ = enigo.key(k, Direction::Press);
    }
}

pub fn key_up(key: &str) {
    let mut enigo = ENIGO.lock().unwrap();

    if let Some(k) = string_to_key(key) {
        let _ = enigo.key(k, Direction::Release);
    }
}

fn string_to_key(key: &str) -> Option<Key> {
    match key.to_lowercase().as_str() {
        // Low octave
        "z" => Some(Key::Unicode('z')),
        "x" => Some(Key::Unicode('x')),
        "c" => Some(Key::Unicode('c')),
        "v" => Some(Key::Unicode('v')),
        "b" => Some(Key::Unicode('b')),
        "n" => Some(Key::Unicode('n')),
        "m" => Some(Key::Unicode('m')),

        // Mid octave
        "a" => Some(Key::Unicode('a')),
        "s" => Some(Key::Unicode('s')),
        "d" => Some(Key::Unicode('d')),
        "f" => Some(Key::Unicode('f')),
        "g" => Some(Key::Unicode('g')),
        "h" => Some(Key::Unicode('h')),
        "j" => Some(Key::Unicode('j')),

        // High octave
        "q" => Some(Key::Unicode('q')),
        "w" => Some(Key::Unicode('w')),
        "e" => Some(Key::Unicode('e')),
        "r" => Some(Key::Unicode('r')),
        "t" => Some(Key::Unicode('t')),
        "y" => Some(Key::Unicode('y')),
        "u" => Some(Key::Unicode('u')),

        _ => None,
    }
}

#[cfg(target_os = "windows")]
pub fn is_black_desert_focused() -> Result<bool, String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Ok(false);
        }
        Ok(matches_target_window(hwnd))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_black_desert_focused() -> Result<bool, String> {
    // For non-Windows platforms, always return true for now
    Ok(true)
}

#[cfg(target_os = "windows")]
pub fn focus_black_desert_window() -> Result<(), String> {
    unsafe {
        let mut data = EnumData { target: None };
        EnumWindows(Some(enum_windows_proc), LPARAM(&mut data as *mut _ as isize))
            .map_err(|e| e.to_string())?;

        if let Some(hwnd) = data.target {
            ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(std::time::Duration::from_millis(50));
            SetForegroundWindow(hwnd);
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(())
        } else {
            Err("WWM window not found".into())
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn focus_black_desert_window() -> Result<(), String> {
    Ok(())
}
