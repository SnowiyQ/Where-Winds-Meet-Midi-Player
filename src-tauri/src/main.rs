#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, State, Window};
use serde::{Serialize, Deserialize};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, MOD_NOREPEAT, VK_END, VK_F9, VK_F10, VK_F11, VK_F12,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, SetWindowsHookExW, CallNextHookEx, TranslateMessage, DispatchMessageW,
    MSG, WM_HOTKEY, WM_KEYDOWN, WM_SYSKEYDOWN, HHOOK, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, HIGH_PRIORITY_CLASS};

// Global app handle for low-level hook callback
static mut GLOBAL_APP_HANDLE: Option<AppHandle> = None;

// Global album path (None = default to exe_dir/album)
use std::sync::RwLock;
static ALBUM_PATH: RwLock<Option<String>> = RwLock::new(None);

fn get_config_path() -> Result<std::path::PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    Ok(exe_dir.join("config.json"))
}

fn load_saved_album_path() {
    if let Ok(config_path) = get_config_path() {
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(path) = json["album_path"].as_str() {
                        let path_buf = std::path::PathBuf::from(path);
                        if path_buf.exists() {
                            if let Ok(mut guard) = ALBUM_PATH.write() {
                                *guard = Some(path.to_string());
                                println!("Loaded album path: {}", path);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn save_album_path(path: Option<&str>) {
    if let Ok(config_path) = get_config_path() {
        let json = match path {
            Some(p) => serde_json::json!({ "album_path": p }),
            None => serde_json::json!({}),
        };
        if let Ok(content) = serde_json::to_string_pretty(&json) {
            let _ = std::fs::write(&config_path, content);
        }
    }
}

fn get_album_folder() -> Result<std::path::PathBuf, String> {
    // Check if custom path is set
    if let Ok(guard) = ALBUM_PATH.read() {
        if let Some(ref custom_path) = *guard {
            let path = std::path::PathBuf::from(custom_path);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    // Default to exe_dir/album
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    Ok(exe_dir.join("album"))
}

mod midi;
mod keyboard;
mod state;

use state::{AppState, PlaybackState, VisualizerNote};

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

// Load MIDI files from album folder
#[tauri::command]
async fn load_midi_files() -> Result<Vec<MidiFile>, String> {
    let album_path = get_album_folder()?;

    let mut files = Vec::new();

    if album_path.exists() {
        let entries = std::fs::read_dir(&album_path).map_err(|e| e.to_string())?;

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
async fn set_note_mode(
    mode: midi::NoteMode,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.set_note_mode(mode);
    println!("Note mode set to: {:?}", mode);
    Ok(())
}

#[tauri::command]
async fn get_note_mode(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<midi::NoteMode, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.get_note_mode())
}

#[tauri::command]
async fn set_octave_shift(
    shift: i8,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.set_octave_shift(shift);
    println!("Octave shift set to: {}", shift);
    Ok(())
}

#[tauri::command]
async fn get_octave_shift(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<i8, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.get_octave_shift())
}

#[tauri::command]
async fn set_speed(
    speed: f64,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.set_speed(speed);
    println!("Speed set to: {}x", speed);
    Ok(())
}

#[tauri::command]
async fn get_speed(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<f64, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.get_speed())
}

#[tauri::command]
async fn set_key_mode(
    mode: midi::KeyMode,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.set_key_mode(mode);
    println!("Key mode set to: {:?}", mode);
    Ok(())
}

#[tauri::command]
async fn get_key_mode(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<midi::KeyMode, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.get_key_mode())
}

#[tauri::command]
async fn is_game_focused() -> Result<bool, String> {
    keyboard::is_wwm_focused().map_err(|e| e.to_string())
}

#[tauri::command]
async fn is_game_window_found() -> Result<bool, String> {
    Ok(keyboard::is_game_window_found())
}

#[tauri::command]
async fn set_modifier_delay(delay_ms: u64) -> Result<(), String> {
    keyboard::set_modifier_delay(delay_ms);
    println!("Modifier delay set to: {}ms", delay_ms);
    Ok(())
}

#[tauri::command]
async fn get_modifier_delay() -> Result<u64, String> {
    Ok(keyboard::get_modifier_delay())
}

#[tauri::command]
async fn set_cloud_mode(enabled: bool) -> Result<(), String> {
    keyboard::set_send_input_mode(enabled);
    Ok(())
}

#[tauri::command]
async fn get_cloud_mode() -> Result<bool, String> {
    Ok(keyboard::get_send_input_mode())
}

#[tauri::command]
async fn test_all_keys() -> Result<(), String> {
    // Test all 21 keys: Low (Z-M), Mid (A-J), High (Q-U)
    let keys = ["z", "x", "c", "v", "b", "n", "m", "a", "s", "d", "f", "g", "h", "j", "q", "w", "e", "r", "t", "y", "u"];
    for key in keys {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    Ok(())
}

#[tauri::command]
async fn test_all_keys_36() -> Result<(), String> {
    // Test all 36 keys including modifiers
    // 21 normal keys + 9 shift keys (sharps) + 6 ctrl keys (flats)

    // Low octave - natural notes
    let low_natural = ["z", "x", "c", "v", "b", "n", "m"];
    // Mid octave - natural notes
    let mid_natural = ["a", "s", "d", "f", "g", "h", "j"];
    // High octave - natural notes
    let high_natural = ["q", "w", "e", "r", "t", "y", "u"];

    // Sharps (Shift+key): #1, #4, #5 per octave
    let low_sharps = ["shift+z", "shift+v", "shift+b"];  // C#, F#, G#
    let mid_sharps = ["shift+a", "shift+f", "shift+g"];
    let high_sharps = ["shift+q", "shift+r", "shift+t"];

    // Flats (Ctrl+key): b3, b7 per octave
    let low_flats = ["ctrl+c", "ctrl+m"];  // Eb, Bb
    let mid_flats = ["ctrl+d", "ctrl+j"];
    let high_flats = ["ctrl+e", "ctrl+u"];

    println!("Testing 36-key mode...");

    // Test low octave
    println!("Low octave - natural notes:");
    for key in low_natural {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("Low octave - sharps:");
    for key in low_sharps {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("Low octave - flats:");
    for key in low_flats {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Test mid octave
    println!("Mid octave - natural notes:");
    for key in mid_natural {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("Mid octave - sharps:");
    for key in mid_sharps {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("Mid octave - flats:");
    for key in mid_flats {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Test high octave
    println!("High octave - natural notes:");
    for key in high_natural {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("High octave - sharps:");
    for key in high_sharps {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("High octave - flats:");
    for key in high_flats {
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_millis(100));
        keyboard::key_up(key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("36-key test complete!");
    Ok(())
}

/// Spam test - rapidly press keys to test PostMessage reliability
/// delay_ms: delay between each key press (0 = max speed)
#[tauri::command]
fn spam_test(key: String, count: u32, delay_ms: u64) -> Result<(), String> {
    println!("[SPAM] Starting: key='{}' count={} delay={}ms", key, count, delay_ms);

    for _ in 0..count {
        keyboard::key_down(&key);
        std::thread::sleep(std::time::Duration::from_micros(500));
        keyboard::key_up(&key);

        if delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }

    println!("[SPAM] Complete! {} keys sent", count);
    Ok(())
}

/// Multi-key spam test - rapidly press multiple different keys
#[tauri::command]
fn spam_test_multi(count: u32, delay_ms: u64) -> Result<(), String> {
    let keys = [
        "z", "x", "c", "v", "b", "n", "m",  // Low
        "a", "s", "d", "f", "g", "h", "j",  // Mid
        "q", "w", "e", "r", "t", "y", "u",  // High
    ];
    println!("[SPAM-MULTI] Starting: {} iterations, delay={}ms, 21 keys", count, delay_ms);

    for i in 0..count {
        let key = keys[i as usize % keys.len()];
        keyboard::key_down(key);
        std::thread::sleep(std::time::Duration::from_micros(500));
        keyboard::key_up(key);

        if delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }

    println!("[SPAM-MULTI] Complete! {} keys sent", count);
    Ok(())
}

/// Chord test - press multiple keys at the SAME time
#[tauri::command]
fn spam_test_chord(chord_size: u32, count: u32, delay_ms: u64) -> Result<(), String> {
    let keys = [
        "z", "x", "c", "v", "b", "n", "m",  // Low
        "a", "s", "d", "f", "g", "h", "j",  // Mid
        "q", "w", "e", "r", "t", "y", "u",  // High
    ];
    let size = chord_size.min(21) as usize;
    println!("[CHORD] Starting: {} notes per chord, {} chords, delay={}ms", size, count, delay_ms);

    for c in 0..count {
        // Press all keys in chord
        for i in 0..size {
            let key_idx = (c as usize * size + i) % 21;
            keyboard::key_down(keys[key_idx]);
        }

        // Hold chord
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Release all keys in chord
        for i in 0..size {
            let key_idx = (c as usize * size + i) % 21;
            keyboard::key_up(keys[key_idx]);
        }

        if delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }

    println!("[CHORD] Complete! {} chords sent", count);
    Ok(())
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
    let album_path = get_album_folder()?;

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
async fn get_album_path() -> Result<String, String> {
    let path = get_album_folder()?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn set_album_path(path: String) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }
    if !path_buf.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    if let Ok(mut guard) = ALBUM_PATH.write() {
        *guard = Some(path.clone());
    }
    save_album_path(Some(&path));
    Ok(())
}

#[tauri::command]
async fn reset_album_path() -> Result<String, String> {
    if let Ok(mut guard) = ALBUM_PATH.write() {
        *guard = None;
    }
    save_album_path(None);
    // Return the default path
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    Ok(exe_dir.join("album").to_string_lossy().to_string())
}

#[tauri::command]
async fn get_visualizer_notes(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<Vec<VisualizerNote>, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.get_visualizer_notes())
}

#[tauri::command]
async fn download_midi_from_url(url: String) -> Result<MidiFile, String> {
    use std::io::Read;

    // Validate URL
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Invalid URL format".to_string());
    }

    // Try to extract filename from URL
    let url_path = url.split('?').next().unwrap_or(&url);
    let filename = url_path
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty() && s.ends_with(".mid"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Generate filename from timestamp if no valid filename in URL
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("download_{}.mid", timestamp)
        });

    // Download the file
    let response = ureq::get(&url)
        .call()
        .map_err(|e| format!("Failed to download: {}", e))?;

    // Check content type or status
    let status = response.status();
    if status != 200 {
        return Err(format!("Server returned status {}", status));
    }

    // Read response body
    let mut bytes = Vec::new();
    response.into_reader()
        .take(10 * 1024 * 1024) // Limit to 10MB
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Validate it looks like a MIDI file (starts with "MThd")
    if bytes.len() < 4 || &bytes[0..4] != b"MThd" {
        return Err("Downloaded file is not a valid MIDI file".to_string());
    }

    // Get album folder path
    let album_path = get_album_folder()?;

    // Create album folder if it doesn't exist
    if !album_path.exists() {
        std::fs::create_dir_all(&album_path).map_err(|e| e.to_string())?;
    }

    // Create destination path
    let dest_path = album_path.join(&filename);

    // Check if file already exists, generate unique name if needed
    let final_path = if dest_path.exists() {
        let stem = filename.trim_end_matches(".mid");
        let mut counter = 1;
        loop {
            let new_name = format!("{}_{}.mid", stem, counter);
            let new_path = album_path.join(&new_name);
            if !new_path.exists() {
                break new_path;
            }
            counter += 1;
            if counter > 100 {
                return Err("Too many files with same name".to_string());
            }
        }
    } else {
        dest_path
    };

    // Write file
    std::fs::write(&final_path, &bytes)
        .map_err(|e| format!("Failed to save file: {}", e))?;

    // Get duration and return file info
    let name = final_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let duration = midi::get_midi_duration(&final_path.to_string_lossy())
        .unwrap_or(0.0);

    Ok(MidiFile {
        name,
        path: final_path.to_string_lossy().to_string(),
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

#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

// ============ Auto-Updater ============

#[derive(Debug, Serialize, Deserialize)]
struct UpdateInfo {
    version: String,
    download_url: String,
    release_url: String,
    file_name: String,
}

#[tauri::command]
async fn check_for_update(current_version: String) -> Result<Option<UpdateInfo>, String> {
    use std::io::Read;

    let response = ureq::get("https://api.github.com/repos/SnowiyQ/Where-Winds-Meet-Midi-Player/releases/latest")
        .set("User-Agent", "WWM-Overlay")
        .call()
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    let mut body = String::new();
    response.into_reader()
        .take(1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| e.to_string())?;

    let latest_version = json["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    if latest_version.is_empty() {
        return Ok(None);
    }

    // Compare versions
    if !is_newer_version(&latest_version, &current_version) {
        return Ok(None);
    }

    // Find the zip asset
    let assets = json["assets"].as_array();
    let download_url = assets
        .and_then(|arr| {
            arr.iter().find(|a| {
                a["name"].as_str()
                    .map(|n| n.ends_with(".zip"))
                    .unwrap_or(false)
            })
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .map(|s| s.to_string());

    let file_name = assets
        .and_then(|arr| {
            arr.iter().find(|a| {
                a["name"].as_str()
                    .map(|n| n.ends_with(".zip"))
                    .unwrap_or(false)
            })
        })
        .and_then(|a| a["name"].as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("wwm-overlay-{}.zip", latest_version));

    let release_url = json["html_url"]
        .as_str()
        .unwrap_or("https://github.com/SnowiyQ/Where-Winds-Meet-Midi-Player/releases/latest")
        .to_string();

    match download_url {
        Some(url) => Ok(Some(UpdateInfo {
            version: latest_version,
            download_url: url,
            release_url,
            file_name,
        })),
        None => Ok(None),
    }
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for i in 0..latest_parts.len().max(current_parts.len()) {
        let l = latest_parts.get(i).unwrap_or(&0);
        let c = current_parts.get(i).unwrap_or(&0);
        if l > c { return true; }
        if l < c { return false; }
    }
    false
}

#[tauri::command]
async fn download_update(download_url: String, file_name: String) -> Result<String, String> {
    use std::io::Read;

    // Download to temp directory
    let temp_dir = std::env::temp_dir();
    let download_path = temp_dir.join(&file_name);

    println!("[UPDATE] Downloading from: {}", download_url);
    println!("[UPDATE] Saving to: {:?}", download_path);

    let response = ureq::get(&download_url)
        .set("User-Agent", "WWM-Overlay")
        .call()
        .map_err(|e| format!("Failed to download update: {}", e))?;

    let mut bytes = Vec::new();
    response.into_reader()
        .take(100 * 1024 * 1024) // 100MB limit
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read download: {}", e))?;

    std::fs::write(&download_path, &bytes)
        .map_err(|e| format!("Failed to save update: {}", e))?;

    println!("[UPDATE] Downloaded {} bytes", bytes.len());

    Ok(download_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn install_update(zip_path: String, app_handle: AppHandle) -> Result<(), String> {
    println!("[UPDATE] Installing from: {}", zip_path);

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get exe directory")?;

    // Create update script that will:
    // 1. Wait for app to close
    // 2. Extract zip over current installation
    // 3. Restart the app
    let script_path = std::env::temp_dir().join("wwm_update.bat");
    let script_content = format!(
        r#"@echo off
echo Updating WWM Overlay...
timeout /t 2 /nobreak > nul
powershell -Command "Expand-Archive -Path '{}' -DestinationPath '{}' -Force"
echo Update complete! Restarting...
start "" "{}"
del "%~f0"
"#,
        zip_path.replace("\\", "\\\\"),
        exe_dir.to_string_lossy().replace("\\", "\\\\"),
        exe_path.to_string_lossy().replace("\\", "\\\\")
    );

    std::fs::write(&script_path, &script_content)
        .map_err(|e| format!("Failed to create update script: {}", e))?;

    println!("[UPDATE] Created update script at: {:?}", script_path);

    // Start the update script
    std::process::Command::new("cmd")
        .args(["/C", "start", "", "/MIN", script_path.to_str().unwrap()])
        .spawn()
        .map_err(|e| format!("Failed to start update script: {}", e))?;

    // Exit the app
    println!("[UPDATE] Exiting for update...");
    app_handle.exit(0);

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
    }

    results
}

// Virtual key codes for [ and ]
const VK_OEM_4: u32 = 0xDB; // [ key
const VK_OEM_6: u32 = 0xDD; // ] key
const VK_INSERT: u32 = 0x2D; // Insert key

// Low-level keyboard hook callback for F12, mode switching, and mini mode
unsafe extern "system" fn low_level_keyboard_proc(
    ncode: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if ncode >= 0 {
        let kb_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let is_keydown = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;

        if is_keydown {
            if let Some(ref app_handle) = GLOBAL_APP_HANDLE {
                // Check if F12 was pressed
                if kb_struct.vkCode == VK_F12.0 as u32 {
                    let _ = app_handle.emit("global-shortcut", "stop");
                }
                // Check if [ was pressed - previous mode
                else if kb_struct.vkCode == VK_OEM_4 {
                    let _ = app_handle.emit("global-shortcut", "mode_prev");
                }
                // Check if ] was pressed - next mode
                else if kb_struct.vkCode == VK_OEM_6 {
                    let _ = app_handle.emit("global-shortcut", "mode_next");
                }
                // Check if Insert was pressed - toggle mini mode
                else if kb_struct.vkCode == VK_INSERT {
                    let _ = app_handle.emit("global-shortcut", "toggle_mini");
                }
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
                        HOTKEY_PREV_F10 => "previous",
                        HOTKEY_NEXT_F11 => "next",
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

/// Set process priority to HIGH for better timing accuracy
fn set_high_priority() {
    unsafe {
        let process = GetCurrentProcess();
        if SetPriorityClass(process, HIGH_PRIORITY_CLASS).is_ok() {
            println!("Process priority set to HIGH");
        } else {
            eprintln!("Failed to set process priority to HIGH");
        }
    }
}

fn main() {
    // Set high priority for accurate MIDI timing
    set_high_priority();

    // Load saved album path from config
    load_saved_album_path();

    let app_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            set_note_mode,
            get_note_mode,
            set_key_mode,
            get_key_mode,
            set_octave_shift,
            get_octave_shift,
            set_speed,
            get_speed,
            set_modifier_delay,
            get_modifier_delay,
            set_cloud_mode,
            get_cloud_mode,
            is_game_focused,
            is_game_window_found,
            test_all_keys,
            test_all_keys_36,
            spam_test,
            spam_test_multi,
            spam_test_chord,
            set_interaction_mode,
            focus_game_window,
            seek,
            import_midi_file,
            download_midi_from_url,
            get_visualizer_notes,
            open_url,
            get_album_path,
            set_album_path,
            reset_album_path,
            check_for_update,
            download_update,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
