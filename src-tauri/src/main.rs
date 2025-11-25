#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, State, Window};
use serde::{Serialize, Deserialize};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, MOD_CONTROL, MOD_NOREPEAT, VK_END, VK_F9, VK_F10, VK_F11, VK_F12,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, SetWindowsHookExW, CallNextHookEx, TranslateMessage, DispatchMessageW,
    MSG, WM_HOTKEY, WM_KEYDOWN, WM_SYSKEYDOWN, HHOOK, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
};
use windows::Win32::Foundation::LPARAM;

// Global app handle for low-level hook callback
static mut GLOBAL_APP_HANDLE: Option<AppHandle> = None;

mod midi;
mod keyboard;
mod state;

use state::{AppState, PlaybackState};

#[derive(Debug, Serialize, Deserialize)]
struct MidiFile {
    name: String,
    path: String,
    duration: f64,
}

// Hotkey IDs
const HOTKEY_PAUSE_RESUME: i32 = 1;
const HOTKEY_STOP_END: i32 = 2;
const HOTKEY_STOP_F12: i32 = 3;
const HOTKEY_PREV_F10: i32 = 4;
const HOTKEY_NEXT_F11: i32 = 5;
const HOTKEY_PREV_CTRL_P: i32 = 6;
const HOTKEY_NEXT_CTRL_N: i32 = 7;
const HOTKEY_LOOP_CTRL_L: i32 = 8;

// Load MIDI files from album folder
#[tauri::command]
async fn load_midi_files() -> Result<Vec<MidiFile>, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    let album_path = exe_dir.join("album");

    let mut files = Vec::new();

    if album_path.exists() {
        let entries = std::fs::read_dir(album_path).map_err(|e| e.to_string())?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("mid") {
                    let name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    // Get actual duration from MIDI file
                    let duration = midi::get_midi_duration(&path.to_string_lossy())
                        .unwrap_or(0.0);

                    files.push(MidiFile {
                        name,
                        path: path.to_string_lossy().to_string(),
                        duration,
                    });
                }
            }
        }
    }

    Ok(files)
}

#[tauri::command]
async fn play_midi(
    path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    window: Window
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.stop_playback();
    app_state.load_midi(&path)?;
    app_state.start_playback(window)?;
    drop(app_state);

    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = keyboard::focus_black_desert_window();

    Ok(())
}

#[tauri::command]
async fn pause_resume(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<PlaybackState, String> {
    let mut app_state = state.lock().unwrap();
    app_state.toggle_pause();
    Ok(app_state.get_playback_state())
}

#[tauri::command]
async fn stop_playback(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.stop_playback();
    Ok(())
}

#[tauri::command]
async fn get_playback_status(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<PlaybackState, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.get_playback_state())
}

#[tauri::command]
async fn set_loop_mode(
    enabled: bool,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.set_loop_mode(enabled);
    Ok(())
}

#[tauri::command]
async fn is_game_focused() -> Result<bool, String> {
    keyboard::is_black_desert_focused().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_interaction_mode(window: Window, interactive: bool) -> Result<(), String> {
    window.set_ignore_cursor_events(!interactive).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn focus_game_window() -> Result<(), String> {
    keyboard::focus_black_desert_window().map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_midi_file(source_path: String) -> Result<MidiFile, String> {
    let source = std::path::Path::new(&source_path);

    // Verify it's a .mid file
    if source.extension().and_then(|s| s.to_str()) != Some("mid") {
        return Err("File must be a .mid file".to_string());
    }

    // Get album folder path
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    let album_path = exe_dir.join("album");

    // Create album folder if it doesn't exist
    if !album_path.exists() {
        std::fs::create_dir_all(&album_path).map_err(|e| e.to_string())?;
    }

    // Get filename and create destination path
    let filename = source.file_name().ok_or("Invalid filename")?;
    let dest_path = album_path.join(filename);

    // Check if file already exists
    if dest_path.exists() {
        return Err(format!("File '{}' already exists in album", filename.to_string_lossy()));
    }

    // Copy file to album folder
    std::fs::copy(&source, &dest_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    // Get duration and return file info
    let name = source.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let duration = midi::get_midi_duration(&dest_path.to_string_lossy())
        .unwrap_or(0.0);

    Ok(MidiFile {
        name,
        path: dest_path.to_string_lossy().to_string(),
        duration,
    })
}

#[tauri::command]
async fn seek(
    position: f64,
    state: State<'_, Arc<Mutex<AppState>>>,
    window: Window
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.seek(position, window)?;
    Ok(())
}

fn register_global_hotkeys() -> Vec<(&'static str, bool)> {
    let mut results = Vec::new();

    unsafe {
        // F9 - Pause/Resume
        let result = RegisterHotKey(None, HOTKEY_PAUSE_RESUME, MOD_NOREPEAT, VK_F9.0 as u32);
        results.push(("F9 (Pause/Resume)", result.is_ok()));

        // End - Stop
        let result = RegisterHotKey(None, HOTKEY_STOP_END, MOD_NOREPEAT, VK_END.0 as u32);
        results.push(("End (Stop)", result.is_ok()));

        // F12 - Stop (may fail if another app has it registered)
        let result = RegisterHotKey(None, HOTKEY_STOP_F12, MOD_NOREPEAT, VK_F12.0 as u32);
        results.push(("F12 (Stop)", result.is_ok()));

        // F10 - Previous
        let result = RegisterHotKey(None, HOTKEY_PREV_F10, MOD_NOREPEAT, VK_F10.0 as u32);
        results.push(("F10 (Previous)", result.is_ok()));

        // F11 - Next
        let result = RegisterHotKey(None, HOTKEY_NEXT_F11, MOD_NOREPEAT, VK_F11.0 as u32);
        results.push(("F11 (Next)", result.is_ok()));

        // Ctrl+P - Previous
        let result = RegisterHotKey(None, HOTKEY_PREV_CTRL_P, MOD_CONTROL | MOD_NOREPEAT, 0x50);
        results.push(("Ctrl+P (Previous)", result.is_ok()));

        // Ctrl+N - Next
        let result = RegisterHotKey(None, HOTKEY_NEXT_CTRL_N, MOD_CONTROL | MOD_NOREPEAT, 0x4E);
        results.push(("Ctrl+N (Next)", result.is_ok()));

        // Ctrl+L - Loop
        let result = RegisterHotKey(None, HOTKEY_LOOP_CTRL_L, MOD_CONTROL | MOD_NOREPEAT, 0x4C);
        results.push(("Ctrl+L (Loop)", result.is_ok()));
    }

    results
}

// Low-level keyboard hook callback for F12 (since RegisterHotKey often fails for F12)
unsafe extern "system" fn low_level_keyboard_proc(
    ncode: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if ncode >= 0 {
        let kb_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let is_keydown = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;

        // Check if F12 was pressed
        if is_keydown && kb_struct.vkCode == VK_F12.0 as u32 {
            if let Some(ref app_handle) = GLOBAL_APP_HANDLE {
                let _ = app_handle.emit("global-shortcut", "stop");
            }
        }
    }

    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

fn start_hotkey_listener(app_handle: AppHandle) {
    // Store app handle globally for the low-level hook callback
    unsafe {
        GLOBAL_APP_HANDLE = Some(app_handle.clone());
    }

    thread::spawn(move || {
        // Register hotkeys in this thread (they will be associated with this thread's message queue)
        let hotkey_results = register_global_hotkeys();

        // Log results
        println!("=== Global Hotkey Registration ===");
        for (name, success) in &hotkey_results {
            if *success {
                println!("  ✓ {}", name);
            } else {
                println!("  ✗ {} (failed - may be in use by another app)", name);
            }
        }
        println!("==================================");

        // Install low-level keyboard hook for F12 as fallback
        unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                None,
                0,
            );

            if hook.is_err() {
                eprintln!("Failed to install low-level keyboard hook for F12");
            } else {
                println!("  ✓ Low-level keyboard hook installed (F12 fallback)");
            }
        }

        // Run message loop to receive hotkey and hook messages
        unsafe {
            let mut msg: MSG = std::mem::zeroed();

            loop {
                // GetMessageW blocks until a message is available
                // For low-level hooks, we need to call it even if no hotkeys registered
                let result = GetMessageW(&mut msg, None, 0, 0);

                if result.0 == -1 {
                    eprintln!("GetMessageW error");
                    break;
                }
                if result.0 == 0 {
                    // WM_QUIT received
                    break;
                }

                if msg.message == WM_HOTKEY {
                    let hotkey_id = msg.wParam.0 as i32;

                    let action = match hotkey_id {
                        HOTKEY_PAUSE_RESUME => "pause_resume",
                        HOTKEY_STOP_END | HOTKEY_STOP_F12 => "stop",
                        HOTKEY_PREV_F10 | HOTKEY_PREV_CTRL_P => "previous",
                        HOTKEY_NEXT_F11 | HOTKEY_NEXT_CTRL_N => "next",
                        HOTKEY_LOOP_CTRL_L => "toggle_loop",
                        _ => continue,
                    };

                    let _ = app_handle.emit("global-shortcut", action);
                }

                // Dispatch other messages (needed for low-level hook to work)
                windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }
        }
    });
}

fn main() {
    let app_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            start_hotkey_listener(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_midi_files,
            play_midi,
            pause_resume,
            stop_playback,
            get_playback_status,
            set_loop_mode,
            is_game_focused,
            set_interaction_mode,
            focus_game_window,
            seek,
            import_midi_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
