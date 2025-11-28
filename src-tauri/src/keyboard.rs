// Virtual keyboard input using PostMessage to game window
// Sends WM_KEYDOWN/WM_KEYUP directly - doesn't affect other apps!

use std::sync::atomic::{AtomicU64, AtomicIsize, AtomicBool, Ordering};
use std::time::{Instant, Duration};
use std::sync::Mutex;

// Configurable modifier delay in milliseconds (default 2ms)
static MODIFIER_DELAY_MS: AtomicU64 = AtomicU64::new(2);

// Input mode: false = PostMessage (default), true = SendInput (for cloud gaming)
static USE_SEND_INPUT: AtomicBool = AtomicBool::new(false);

/// Set input mode: true = SendInput (cloud gaming), false = PostMessage (local)
pub fn set_send_input_mode(enabled: bool) {
    USE_SEND_INPUT.store(enabled, Ordering::SeqCst);
    println!("[KEYBOARD] Input mode: {}", if enabled { "SendInput (cloud)" } else { "PostMessage (local)" });
}

/// Get current input mode
pub fn get_send_input_mode() -> bool {
    USE_SEND_INPUT.load(Ordering::SeqCst)
}

// Cached window handle and last check time
static CACHED_HWND: AtomicIsize = AtomicIsize::new(0);
lazy_static::lazy_static! {
    static ref LAST_WINDOW_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
}
const WINDOW_CACHE_DURATION: Duration = Duration::from_secs(5);

/// Set the delay between modifier key and main key press
pub fn set_modifier_delay(delay_ms: u64) {
    MODIFIER_DELAY_MS.store(delay_ms, Ordering::SeqCst);
}

/// Get the current modifier delay
pub fn get_modifier_delay() -> u64 {
    MODIFIER_DELAY_MS.load(Ordering::SeqCst)
}

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows,
    GetForegroundWindow,
    GetWindowTextW,
    SetForegroundWindow,
    ShowWindow,
    PostMessageW,
    SW_RESTORE,
    WM_KEYDOWN,
    WM_KEYUP,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
    MapVirtualKeyW, MAPVK_VK_TO_VSC,
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};


#[cfg(target_os = "windows")]
const TARGET_WINDOW_KEYWORDS: [&str; 6] =
    ["where winds meet", "wwm", "wwm.exe", "geforce now", "geforcenow", "nvidia geforce"];

#[cfg(target_os = "windows")]
struct EnumData {
    target: Option<HWND>,
}

#[cfg(target_os = "windows")]
fn matches_target_window(hwnd: HWND, log: bool) -> bool {
    let mut title = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut title) };
    if len <= 0 {
        return false;
    }
    let title_string = String::from_utf16_lossy(&title[..len as usize]).to_lowercase();

    // Skip our own window and common apps that should never receive keys
    if title_string.contains("midi player")
        || title_string.contains("overlay")
        || title_string.contains("discord")
        || title_string.contains("telegram")
        || title_string.contains("slack")
        || title_string.contains("teams")
        || title_string.contains("notepad")
        || title_string.contains("visual studio")
        || title_string.contains("vscode")
    {
        return false;
    }

    let matched_keyword = TARGET_WINDOW_KEYWORDS
        .iter()
        .find(|keyword| title_string.contains(*keyword));
    if let Some(keyword) = matched_keyword {
        if log {
            println!("[WINDOW] Found matching window: '{}' (matched: '{}') hwnd={:?}", title_string, keyword, hwnd.0);
        }
        true
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut EnumData);
    if matches_target_window(hwnd, true) {
        data.target = Some(hwnd);
        return BOOL(0);
    }
    BOOL(1)
}

// ============ Background keyboard injection ============
// Attaches to game thread, focuses game, sends input, restores focus
// This allows sending keys to game while doing other things

/// Find game window (with caching to avoid repeated searches)
#[cfg(target_os = "windows")]
fn find_game_window() -> Option<HWND> {
    // Check if we have a valid cached handle
    let cached = CACHED_HWND.load(Ordering::SeqCst);
    let mut last_check = LAST_WINDOW_CHECK.lock().unwrap();

    let should_refresh = if cached == 0 {
        true
    } else if let Some(last) = *last_check {
        last.elapsed() > WINDOW_CACHE_DURATION
    } else {
        true
    };

    if !should_refresh && cached != 0 {
        return Some(HWND(cached as *mut std::ffi::c_void));
    }

    // Search for window
    unsafe {
        let mut data = EnumData { target: None };
        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(&mut data as *mut _ as isize));

        if let Some(hwnd) = data.target {
            // Cache the result
            CACHED_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
            *last_check = Some(Instant::now());
            println!("[WINDOW] Cached game window hwnd={:?}", hwnd.0);
            Some(hwnd)
        } else {
            // Clear cache if window not found
            CACHED_HWND.store(0, Ordering::SeqCst);
            *last_check = None;
            None
        }
    }
}

/// Clear the cached window handle (call when starting new playback)
pub fn clear_window_cache() {
    CACHED_HWND.store(0, Ordering::SeqCst);
    if let Ok(mut last_check) = LAST_WINDOW_CHECK.lock() {
        *last_check = None;
    }
}

// Modifier key codes
#[cfg(target_os = "windows")]
const VK_SHIFT: u32 = 0x10;
#[cfg(target_os = "windows")]
const VK_CONTROL: u32 = 0x11;

/// Key with optional modifier
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub enum Modifier {
    None,
    Shift,
    Ctrl,
}

/// Convert key string to virtual key code and modifier
/// Format: "key" for normal, "shift+key" for shift, "ctrl+key" for ctrl
#[cfg(target_os = "windows")]
fn parse_key(key: &str) -> Option<(u32, Modifier)> {
    let key_lower = key.to_lowercase();

    // Check for modifier prefix
    if key_lower.starts_with("shift+") {
        let base_key = &key_lower[6..];
        return key_to_vk(base_key).map(|vk| (vk, Modifier::Shift));
    }
    if key_lower.starts_with("ctrl+") {
        let base_key = &key_lower[5..];
        return key_to_vk(base_key).map(|vk| (vk, Modifier::Ctrl));
    }

    key_to_vk(&key_lower).map(|vk| (vk, Modifier::None))
}

/// Convert key string to virtual key code
#[cfg(target_os = "windows")]
fn key_to_vk(key: &str) -> Option<u32> {
    match key {
        // Low octave: Z X C V B N M
        "z" => Some(0x5A),
        "x" => Some(0x58),
        "c" => Some(0x43),
        "v" => Some(0x56),
        "b" => Some(0x42),
        "n" => Some(0x4E),
        "m" => Some(0x4D),

        // Mid octave: A S D F G H J
        "a" => Some(0x41),
        "s" => Some(0x53),
        "d" => Some(0x44),
        "f" => Some(0x46),
        "g" => Some(0x47),
        "h" => Some(0x48),
        "j" => Some(0x4A),

        // High octave: Q W E R T Y U
        "q" => Some(0x51),
        "w" => Some(0x57),
        "e" => Some(0x45),
        "r" => Some(0x52),
        "t" => Some(0x54),
        "y" => Some(0x59),
        "u" => Some(0x55),

        _ => None,
    }
}

/// Build lParam for WM_KEYDOWN (scan code in bits 16-23)
#[cfg(target_os = "windows")]
fn make_keydown_lparam(vk: u32) -> LPARAM {
    unsafe {
        let scan = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
        // Bits: 0-15 = repeat count (1), 16-23 = scan code, 24 = extended, 29 = context, 30 = prev state, 31 = transition
        let lparam = 1u32 | ((scan & 0xFF) << 16);
        LPARAM(lparam as isize)
    }
}

/// Build lParam for WM_KEYUP (scan code + release flags)
#[cfg(target_os = "windows")]
fn make_keyup_lparam(vk: u32) -> LPARAM {
    unsafe {
        let scan = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
        // Bits 30 and 31 set for key release
        let lparam = 1u32 | ((scan & 0xFF) << 16) | (1 << 30) | (1 << 31);
        LPARAM(lparam as isize)
    }
}

/// Get virtual key code for modifier
#[cfg(target_os = "windows")]
fn modifier_to_vk(modifier: Modifier) -> Option<u32> {
    match modifier {
        Modifier::Shift => Some(VK_SHIFT),
        Modifier::Ctrl => Some(VK_CONTROL),
        Modifier::None => None,
    }
}

/// Reset modifier counts (no-op now, kept for compatibility)
pub fn reset_modifier_counts() {
    // No longer using reference counting
}

// ============ SendInput-based functions (for cloud gaming) ============

#[cfg(target_os = "windows")]
fn send_input_key_down(vk: u32) {
    unsafe {
        let scan_code = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) as u16;
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk as u16),
                    wScan: scan_code,
                    dwFlags: KEYEVENTF_SCANCODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "windows")]
fn send_input_key_up(vk: u32) {
    unsafe {
        let scan_code = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) as u16;
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk as u16),
                    wScan: scan_code,
                    dwFlags: KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

// ============ Key down/up with mode switching ============

#[cfg(target_os = "windows")]
pub fn key_down(key: &str) {
    if let Some((vk, modifier)) = parse_key(key) {
        if USE_SEND_INPUT.load(Ordering::SeqCst) {
            // SendInput mode - global keyboard simulation
            // Only send if a game window is currently focused (prevent typing in Discord etc)
            if !is_wwm_focused().unwrap_or(false) {
                return;
            }
            // Send modifier first if needed
            if let Some(mod_vk) = modifier_to_vk(modifier) {
                send_input_key_down(mod_vk);
                let delay = get_modifier_delay();
                if delay > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
            }
            send_input_key_down(vk);
        } else {
            // PostMessage mode - targeted to game window
            match find_game_window() {
                Some(hwnd) => {
                    unsafe {
                        // Send modifier first if needed
                        if let Some(mod_vk) = modifier_to_vk(modifier) {
                            let mod_lparam = make_keydown_lparam(mod_vk);
                            let _ = PostMessageW(hwnd, WM_KEYDOWN, WPARAM(mod_vk as usize), mod_lparam);
                            let delay = get_modifier_delay();
                            if delay > 0 {
                                std::thread::sleep(std::time::Duration::from_millis(delay));
                            }
                        }
                        let lparam = make_keydown_lparam(vk);
                        let _ = PostMessageW(hwnd, WM_KEYDOWN, WPARAM(vk as usize), lparam);
                    }
                }
                None => {}
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn key_up(key: &str) {
    if let Some((vk, modifier)) = parse_key(key) {
        if USE_SEND_INPUT.load(Ordering::SeqCst) {
            // SendInput mode - global keyboard simulation
            // Only send if a game window is currently focused (prevent typing in Discord etc)
            if !is_wwm_focused().unwrap_or(false) {
                return;
            }
            send_input_key_up(vk);
            if let Some(mod_vk) = modifier_to_vk(modifier) {
                let delay = get_modifier_delay();
                if delay > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
                send_input_key_up(mod_vk);
            }
        } else {
            // PostMessage mode - targeted to game window
            if let Some(hwnd) = find_game_window() {
                unsafe {
                    let lparam = make_keyup_lparam(vk);
                    let _ = PostMessageW(hwnd, WM_KEYUP, WPARAM(vk as usize), lparam);

                    if let Some(mod_vk) = modifier_to_vk(modifier) {
                        let delay = get_modifier_delay();
                        if delay > 0 {
                            std::thread::sleep(std::time::Duration::from_millis(delay));
                        }
                        let mod_lparam = make_keyup_lparam(mod_vk);
                        let _ = PostMessageW(hwnd, WM_KEYUP, WPARAM(mod_vk as usize), mod_lparam);
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn key_down(_key: &str) {
    // Non-Windows: no-op for now
}

#[cfg(not(target_os = "windows"))]
pub fn key_up(_key: &str) {
    // Non-Windows: no-op for now
}

#[cfg(not(target_os = "windows"))]
pub fn reset_modifier_counts() {
    // Non-Windows: no-op
}

#[cfg(not(target_os = "windows"))]
pub fn clear_window_cache() {
    // Non-Windows: no-op
}

// ============ Old Enigo-based method (commented out) ============
/*
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
        "z" => Some(Key::Unicode('z')),
        "x" => Some(Key::Unicode('x')),
        "c" => Some(Key::Unicode('c')),
        "v" => Some(Key::Unicode('v')),
        "b" => Some(Key::Unicode('b')),
        "n" => Some(Key::Unicode('n')),
        "m" => Some(Key::Unicode('m')),
        "a" => Some(Key::Unicode('a')),
        "s" => Some(Key::Unicode('s')),
        "d" => Some(Key::Unicode('d')),
        "f" => Some(Key::Unicode('f')),
        "g" => Some(Key::Unicode('g')),
        "h" => Some(Key::Unicode('h')),
        "j" => Some(Key::Unicode('j')),
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
*/

/// Check if game window exists (for status indicator)
#[cfg(target_os = "windows")]
pub fn is_game_window_found() -> bool {
    find_game_window().is_some()
}

#[cfg(not(target_os = "windows"))]
pub fn is_game_window_found() -> bool {
    true
}

#[cfg(target_os = "windows")]
pub fn is_wwm_focused() -> Result<bool, String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Ok(false);
        }
        Ok(matches_target_window(hwnd, false))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_wwm_focused() -> Result<bool, String> {
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
            Err("Game window not found (WWM or GeForce Now)".into())
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn focus_black_desert_window() -> Result<(), String> {
    Ok(())
}
