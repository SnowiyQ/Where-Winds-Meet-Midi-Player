#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use log::{info, warn, error};
use simplelog::{WriteLogger, CombinedLogger, TermLogger, LevelFilter, Config, ConfigBuilder, TerminalMode, ColorChoice};
use std::fs::File;

/// Log macro that prints to console AND logs to file
#[macro_export]
macro_rules! app_log {
    ($($arg:tt)*) => {{
        println!($($arg)*);
        log::info!($($arg)*);
    }};
}

#[macro_export]
macro_rules! app_error {
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        log::error!($($arg)*);
    }};
}

fn init_logger() {
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("wwm-overlay.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("wwm-overlay.log"));

    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .build();

    if let Ok(file) = File::create(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Info, config, file);
        info!("=== WWM Overlay Started ===");
    }
}
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

// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub pause_resume: String,  // Default: "F9"
    pub stop: String,          // Default: "F12"
    pub previous: String,      // Default: "F10"
    pub next: String,          // Default: "F11"
    pub mode_prev: String,     // Default: "["
    pub mode_next: String,     // Default: "]"
    pub toggle_mini: String,   // Default: "Insert"
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindings {
            pause_resume: "F9".to_string(),
            stop: "F12".to_string(),
            previous: "F10".to_string(),
            next: "F11".to_string(),
            mode_prev: "[".to_string(),
            mode_next: "]".to_string(),
            toggle_mini: "Insert".to_string(),
        }
    }
}

// Global keybindings
static KEYBINDINGS: RwLock<Option<KeyBindings>> = RwLock::new(None);

fn get_keybindings() -> KeyBindings {
    KEYBINDINGS.read().unwrap().clone().unwrap_or_default()
}

fn load_saved_keybindings() {
    let config = load_config();
    if let Some(kb) = config.get("keybindings") {
        if let Ok(keybindings) = serde_json::from_value::<KeyBindings>(kb.clone()) {
            if let Ok(mut guard) = KEYBINDINGS.write() {
                *guard = Some(keybindings);
                app_log!("Loaded custom keybindings");
            }
        }
    }
}

fn save_keybindings(keybindings: &KeyBindings) {
    let mut config = load_config();
    config["keybindings"] = serde_json::to_value(keybindings).unwrap_or_default();
    save_config(&config);
    if let Ok(mut guard) = KEYBINDINGS.write() {
        *guard = Some(keybindings.clone());
    }
}

// Convert key string to virtual key code
fn key_to_vk(key: &str) -> Option<u32> {
    let upper = key.to_uppercase();
    match upper.as_str() {
        // Function keys
        "F1" => Some(0x70), "F2" => Some(0x71), "F3" => Some(0x72), "F4" => Some(0x73),
        "F5" => Some(0x74), "F6" => Some(0x75), "F7" => Some(0x76), "F8" => Some(0x77),
        "F9" => Some(0x78), "F10" => Some(0x79), "F11" => Some(0x7A), "F12" => Some(0x7B),
        // Special keys
        "INSERT" | "INS" => Some(0x2D),
        "DELETE" | "DEL" => Some(0x2E),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGEUP" | "PGUP" => Some(0x21),
        "PAGEDOWN" | "PGDN" => Some(0x22),
        "SCROLLLOCK" => Some(0x91),
        "PAUSE" => Some(0x13),
        "NUMLOCK" => Some(0x90),
        "PRINTSCREEN" => Some(0x2C),
        // Arrow keys
        "UP" | "ARROWUP" => Some(0x26),
        "DOWN" | "ARROWDOWN" => Some(0x28),
        "LEFT" | "ARROWLEFT" => Some(0x25),
        "RIGHT" | "ARROWRIGHT" => Some(0x27),
        // OEM keys (symbols)
        "[" | "OEM_4" => Some(0xDB),
        "]" | "OEM_6" => Some(0xDD),
        "`" | "OEM_3" => Some(0xC0),
        "-" | "OEM_MINUS" => Some(0xBD),
        "=" | "OEM_PLUS" => Some(0xBB),
        "\\" | "OEM_5" => Some(0xDC),
        ";" | "OEM_1" => Some(0xBA),
        "'" | "OEM_7" => Some(0xDE),
        "," | "OEM_COMMA" => Some(0xBC),
        "." | "OEM_PERIOD" => Some(0xBE),
        "/" | "OEM_2" => Some(0xBF),
        // Letters A-Z (VK codes 0x41-0x5A)
        "A" => Some(0x41), "B" => Some(0x42), "C" => Some(0x43), "D" => Some(0x44),
        "E" => Some(0x45), "F" => Some(0x46), "G" => Some(0x47), "H" => Some(0x48),
        "I" => Some(0x49), "J" => Some(0x4A), "K" => Some(0x4B), "L" => Some(0x4C),
        "M" => Some(0x4D), "N" => Some(0x4E), "O" => Some(0x4F), "P" => Some(0x50),
        "Q" => Some(0x51), "R" => Some(0x52), "S" => Some(0x53), "T" => Some(0x54),
        "U" => Some(0x55), "V" => Some(0x56), "W" => Some(0x57), "X" => Some(0x58),
        "Y" => Some(0x59), "Z" => Some(0x5A),
        // Numbers 0-9 (VK codes 0x30-0x39)
        "0" => Some(0x30), "1" => Some(0x31), "2" => Some(0x32), "3" => Some(0x33),
        "4" => Some(0x34), "5" => Some(0x35), "6" => Some(0x36), "7" => Some(0x37),
        "8" => Some(0x38), "9" => Some(0x39),
        // Numpad keys
        "NUMPAD0" => Some(0x60), "NUMPAD1" => Some(0x61), "NUMPAD2" => Some(0x62),
        "NUMPAD3" => Some(0x63), "NUMPAD4" => Some(0x64), "NUMPAD5" => Some(0x65),
        "NUMPAD6" => Some(0x66), "NUMPAD7" => Some(0x67), "NUMPAD8" => Some(0x68),
        "NUMPAD9" => Some(0x69),
        "NUMPADMULTIPLY" => Some(0x6A), "NUMPADADD" => Some(0x6B),
        "NUMPADSUBTRACT" => Some(0x6D), "NUMPADDECIMAL" => Some(0x6E),
        "NUMPADDIVIDE" => Some(0x6F),
        _ => None,
    }
}

fn get_config_path() -> Result<std::path::PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    Ok(exe_dir.join("config.json"))
}

fn load_config() -> serde_json::Value {
    if let Ok(config_path) = get_config_path() {
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str(&content) {
                    return json;
                }
            }
        }
    }
    serde_json::json!({})
}

fn save_config(config: &serde_json::Value) {
    if let Ok(config_path) = get_config_path() {
        if let Ok(content) = serde_json::to_string_pretty(config) {
            let _ = std::fs::write(&config_path, content);
        }
    }
}

fn load_saved_album_path() {
    let config = load_config();
    if let Some(path) = config["album_path"].as_str() {
        let path_buf = std::path::PathBuf::from(path);
        if path_buf.exists() {
            if let Ok(mut guard) = ALBUM_PATH.write() {
                *guard = Some(path.to_string());
                app_log!("Loaded album path: {}", path);
            }
        }
    }
}

fn save_album_path(path: Option<&str>) {
    let mut config = load_config();
    match path {
        Some(p) => config["album_path"] = serde_json::json!(p),
        None => { config.as_object_mut().map(|o| o.remove("album_path")); }
    }
    save_config(&config);
}

fn load_saved_note_keys() {
    let config = load_config();
    if let Some(keys) = config.get("note_keys") {
        let low: Vec<String> = keys["low"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let mid: Vec<String> = keys["mid"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let high: Vec<String> = keys["high"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        if !low.is_empty() && !mid.is_empty() && !high.is_empty() {
            keyboard::set_note_key_bindings(low.clone(), mid.clone(), high.clone());
            app_log!("Loaded note key bindings");
        }
    }
    // Migration: if old qwertz_mode exists, migrate to new format
    else if let Some(enabled) = config["qwertz_mode"].as_bool() {
        if enabled {
            // QWERTZ: swap Y and Z
            let low = vec!["y".to_string(), "x".to_string(), "c".to_string(), "v".to_string(), "b".to_string(), "n".to_string(), "m".to_string()];
            let mid = vec!["a".to_string(), "s".to_string(), "d".to_string(), "f".to_string(), "g".to_string(), "h".to_string(), "j".to_string()];
            let high = vec!["q".to_string(), "w".to_string(), "e".to_string(), "r".to_string(), "t".to_string(), "z".to_string(), "u".to_string()];
            keyboard::set_note_key_bindings(low.clone(), mid.clone(), high.clone());
            save_note_keys(&low, &mid, &high);
            app_log!("Migrated from qwertz_mode to note_keys");
        }
    }
}

fn save_note_keys(low: &[String], mid: &[String], high: &[String]) {
    let mut config = load_config();
    config["note_keys"] = serde_json::json!({
        "low": low,
        "mid": mid,
        "high": high
    });
    // Remove old format if present
    if config.get("qwertz_mode").is_some() {
        config.as_object_mut().unwrap().remove("qwertz_mode");
    }
    save_config(&config);
}

fn load_custom_window_keywords() {
    let config = load_config();
    if let Some(keywords) = config["custom_window_keywords"].as_array() {
        let kw: Vec<String> = keywords
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        keyboard::set_custom_window_keywords(kw);
        app_log!("Loaded custom window keywords");
    }
}

fn save_custom_window_keywords(keywords: &[String]) {
    let mut config = load_config();
    config["custom_window_keywords"] = serde_json::json!(keywords);
    save_config(&config);
}

fn get_data_path(filename: &str) -> Result<std::path::PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get executable directory")?;
    Ok(exe_dir.join(filename))
}

fn get_album_folder() -> Result<std::path::PathBuf, String> {
    // Check if custom path is set - return it even if it doesn't exist yet
    // (the caller will create it if needed)
    if let Ok(guard) = ALBUM_PATH.read() {
        if let Some(ref custom_path) = *guard {
            return Ok(std::path::PathBuf::from(custom_path));
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
mod discovery;

use state::{AppState, PlaybackState, VisualizerNote};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MidiFile {
    name: String,
    path: String,
    duration: f64,
    bpm: u16,
    note_density: f32,
    hash: String,
    size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetadataCache {
    version: u8,
    files: std::collections::HashMap<String, CachedMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedMetadata {
    mtime: u64,  // file modification time
    duration: f64,
    bpm: u16,
    note_density: f32,
}

fn get_metadata_cache_path() -> Result<std::path::PathBuf, String> {
    let album_path = get_album_folder()?;
    Ok(album_path.join(".metadata_cache.json"))
}

fn load_metadata_cache() -> MetadataCache {
    if let Ok(cache_path) = get_metadata_cache_path() {
        if cache_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cache_path) {
                if let Ok(cache) = serde_json::from_str::<MetadataCache>(&content) {
                    if cache.version == 1 {
                        return cache;
                    }
                }
            }
        }
    }
    MetadataCache {
        version: 1,
        files: std::collections::HashMap::new(),
    }
}

fn save_metadata_cache(cache: &MetadataCache) {
    if let Ok(cache_path) = get_metadata_cache_path() {
        if let Ok(content) = serde_json::to_string(cache) {
            let _ = std::fs::write(&cache_path, content);
        }
    }
}

fn get_file_mtime(path: &std::path::Path) -> u64 {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Compute a simple hash of file content for identification
fn compute_file_hash(path: &std::path::Path) -> Option<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;

    // Read first 8KB + file size for quick but reliable hash
    let mut buffer = [0u8; 8192];
    let bytes_read = file.read(&mut buffer).ok()?;

    let file_size = file.metadata().ok()?.len();

    // Simple hash combining file content and size
    let mut hash: u64 = file_size;
    for byte in &buffer[..bytes_read] {
        hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
    }

    Some(format!("{:016x}", hash))
}

// Hotkey IDs
const HOTKEY_PAUSE_RESUME: i32 = 1;
const HOTKEY_STOP_END: i32 = 2;
const HOTKEY_STOP_F12: i32 = 3;
const HOTKEY_PREV_F10: i32 = 4;
const HOTKEY_NEXT_F11: i32 = 5;

// Load MIDI files from album folder with metadata caching
#[tauri::command]
async fn load_midi_files() -> Result<Vec<MidiFile>, String> {
    let album_path = get_album_folder()?;
    let mut files = Vec::new();

    if !album_path.exists() {
        return Ok(files);
    }

    // Load existing cache
    let mut cache = load_metadata_cache();
    let mut cache_modified = false;

    let entries = std::fs::read_dir(&album_path).map_err(|e| e.to_string())?;

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("mid") {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let mtime = get_file_mtime(&path);

            // Check cache
            let metadata = if let Some(cached) = cache.files.get(&path_str) {
                if cached.mtime == mtime {
                    // Cache hit
                    (cached.duration, cached.bpm, cached.note_density)
                } else {
                    // File modified, re-parse
                    let meta = midi::get_midi_metadata(&path_str).unwrap_or(midi::MidiMetadata {
                        duration: 0.0, bpm: 120, note_count: 0, note_density: 0.0
                    });
                    cache.files.insert(path_str.clone(), CachedMetadata {
                        mtime, duration: meta.duration, bpm: meta.bpm, note_density: meta.note_density
                    });
                    cache_modified = true;
                    (meta.duration, meta.bpm, meta.note_density)
                }
            } else {
                // Not in cache, parse
                let meta = midi::get_midi_metadata(&path_str).unwrap_or(midi::MidiMetadata {
                    duration: 0.0, bpm: 120, note_count: 0, note_density: 0.0
                });
                cache.files.insert(path_str.clone(), CachedMetadata {
                    mtime, duration: meta.duration, bpm: meta.bpm, note_density: meta.note_density
                });
                cache_modified = true;
                (meta.duration, meta.bpm, meta.note_density)
            };

            // Compute file hash and size
            let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let file_hash = compute_file_hash(&path).unwrap_or_else(|| format!("{:x}", file_size));

            files.push(MidiFile {
                name,
                path: path_str,
                duration: metadata.0,
                bpm: metadata.1,
                note_density: metadata.2,
                hash: file_hash,
                size: file_size,
            });
        }
    }

    // Save cache if modified
    if cache_modified {
        save_metadata_cache(&cache);
    }

    Ok(files)
}

#[tauri::command]
async fn get_midi_tracks(path: String) -> Result<Vec<midi::MidiTrackInfo>, String> {
    midi::get_midi_tracks(&path)
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
async fn play_midi_band(
    path: String,
    mode: String,
    slot: usize,
    total_players: usize,
    track_id: Option<usize>,
    state: State<'_, Arc<Mutex<AppState>>>,
    window: Window
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.stop_playback();
    app_state.load_midi(&path)?;

    // Set band mode filter before starting playback
    app_state.set_band_filter(mode, slot, total_players, track_id);

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
async fn set_track_filter(
    track_id: Option<usize>,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let app_state = state.lock().unwrap();
    app_state.update_band_filter_live(track_id);
    println!("Track filter set to: {:?}", track_id);
    Ok(())
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
async fn set_note_keys(low: Vec<String>, mid: Vec<String>, high: Vec<String>) -> Result<(), String> {
    keyboard::set_note_key_bindings(low.clone(), mid.clone(), high.clone());
    save_note_keys(&low, &mid, &high);
    Ok(())
}

#[tauri::command]
async fn get_note_keys() -> Result<serde_json::Value, String> {
    let (low, mid, high) = keyboard::get_note_key_bindings();
    Ok(serde_json::json!({
        "low": low,
        "mid": mid,
        "high": high
    }))
}

#[tauri::command]
async fn reset_note_keys() -> Result<serde_json::Value, String> {
    keyboard::reset_note_key_bindings();
    // Clear from config
    let mut config = load_config();
    if config.get("note_keys").is_some() {
        config.as_object_mut().unwrap().remove("note_keys");
        save_config(&config);
    }
    // Return defaults
    let (low, mid, high) = keyboard::get_note_key_bindings();
    Ok(serde_json::json!({
        "low": low,
        "mid": mid,
        "high": high
    }))
}

#[tauri::command]
async fn set_custom_window_keywords(keywords: Vec<String>) -> Result<(), String> {
    keyboard::set_custom_window_keywords(keywords.clone());
    save_custom_window_keywords(&keywords);
    Ok(())
}

#[tauri::command]
async fn get_custom_window_keywords() -> Result<Vec<String>, String> {
    Ok(keyboard::get_custom_window_keywords())
}

#[tauri::command]
async fn cmd_get_keybindings() -> Result<KeyBindings, String> {
    Ok(get_keybindings())
}

#[tauri::command]
async fn cmd_set_keybindings(keybindings: KeyBindings) -> Result<(), String> {
    save_keybindings(&keybindings);
    cache_keybinding_vks(); // Hot reload
    Ok(())
}

#[tauri::command]
async fn cmd_reset_keybindings() -> Result<KeyBindings, String> {
    let default_kb = KeyBindings::default();
    save_keybindings(&default_kb);
    cache_keybinding_vks(); // Hot reload
    Ok(default_kb)
}

#[tauri::command]
async fn cmd_set_keybindings_enabled(enabled: bool) -> Result<(), String> {
    unsafe {
        KEYBINDINGS_DISABLED = !enabled;
        RECORDING_MODE = !enabled;
    }
    Ok(())
}

#[tauri::command]
async fn cmd_unfocus_window() -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, SetForegroundWindow};
    unsafe {
        let desktop = GetDesktopWindow();
        let _ = SetForegroundWindow(desktop);
    }
    Ok(())
}

#[tauri::command]
async fn press_key(key: String) -> Result<(), String> {
    keyboard::key_down(&key);
    keyboard::key_up(&key);
    Ok(())
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

    // All 36 keys in order
    let all_keys: Vec<&str> = [
        low_natural.as_slice(), low_sharps.as_slice(), low_flats.as_slice(),
        mid_natural.as_slice(), mid_sharps.as_slice(), mid_flats.as_slice(),
        high_natural.as_slice(), high_sharps.as_slice(), high_flats.as_slice(),
    ].concat();

    // Test all keys - instant combo (shift+x together), small gap between notes for UI
    for key in all_keys {
        keyboard::key_down(key);  // Shift+X fires together instantly
        keyboard::key_up(key);    // Release together instantly
        std::thread::sleep(std::time::Duration::from_millis(50)); // Gap between notes for UI
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

    // Get metadata and return file info
    let name = source.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let meta = midi::get_midi_metadata(&dest_path.to_string_lossy())
        .unwrap_or(midi::MidiMetadata {
            duration: 0.0, bpm: 120, note_count: 0, note_density: 0.0
        });

    let file_size = std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);
    let file_hash = compute_file_hash(&dest_path).unwrap_or_else(|| format!("{:x}", file_size));

    Ok(MidiFile {
        name,
        path: dest_path.to_string_lossy().to_string(),
        duration: meta.duration,
        bpm: meta.bpm,
        note_density: meta.note_density,
        hash: file_hash,
        size: file_size,
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

// Band mode: Read MIDI file as base64 for transfer
#[tauri::command]
async fn read_midi_base64(path: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let data = std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Verify it's a valid MIDI file (starts with "MThd")
    if data.len() < 4 || &data[0..4] != b"MThd" {
        return Err("Not a valid MIDI file".to_string());
    }

    Ok(STANDARD.encode(&data))
}

// Band mode: Check if MIDI file exists in album folder by name
#[tauri::command]
async fn check_midi_exists(filename: String) -> Result<Option<String>, String> {
    let album_path = get_album_folder()?;
    let file_path = album_path.join(&filename);

    if file_path.exists() && file_path.extension().map(|e| e == "mid").unwrap_or(false) {
        Ok(Some(file_path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

// Band mode: Save MIDI file to temp for playback
#[tauri::command]
async fn save_temp_midi(filename: String, data_base64: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let data = STANDARD.decode(&data_base64)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Verify it's a valid MIDI file
    if data.len() < 4 || &data[0..4] != b"MThd" {
        return Err("Not a valid MIDI file".to_string());
    }

    // Save to temp directory
    let temp_dir = std::env::temp_dir().join("wwm-band");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let temp_path = temp_dir.join(&filename);
    std::fs::write(&temp_path, &data).map_err(|e| format!("Failed to write temp file: {}", e))?;

    Ok(temp_path.to_string_lossy().to_string())
}

// Verify MIDI data is valid (for P2P library safety)
#[tauri::command]
async fn verify_midi_data(data_base64: String) -> Result<bool, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let data = STANDARD.decode(&data_base64)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Check minimum size
    if data.len() < 14 {
        return Ok(false);
    }

    // Check MIDI header "MThd"
    if &data[0..4] != b"MThd" {
        return Ok(false);
    }

    // Check header length (should be 6)
    let header_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if header_len != 6 {
        return Ok(false);
    }

    // Check for at least one track header "MTrk"
    if data.len() < 22 {
        return Ok(false);
    }

    // Find MTrk header (should be at offset 14 after MThd)
    if &data[14..18] != b"MTrk" {
        return Ok(false);
    }

    // Verify file size is reasonable (max 50MB)
    if data.len() > 50 * 1024 * 1024 {
        return Ok(false);
    }

    // Try to parse with midly to ensure it's actually valid
    match midly::Smf::parse(&data) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// Save MIDI file to album folder (for P2P library)
#[tauri::command]
async fn save_midi_from_base64(filename: String, data_base64: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let data = STANDARD.decode(&data_base64)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Verify it's a valid MIDI file
    if data.len() < 14 || &data[0..4] != b"MThd" {
        return Err("Not a valid MIDI file".to_string());
    }

    // Try to parse to ensure it's valid
    midly::Smf::parse(&data).map_err(|e| format!("Invalid MIDI file: {}", e))?;

    // Get album folder
    let album_dir = get_album_folder()?;

    // Sanitize filename (remove path separators, etc.)
    let safe_filename: String = filename
        .chars()
        .filter(|c| !['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(c))
        .collect();

    // Ensure .mid extension
    let final_filename = if safe_filename.to_lowercase().ends_with(".mid") {
        safe_filename
    } else {
        format!("{}.mid", safe_filename)
    };

    // Check if file already exists, add number if so
    let mut save_path = album_dir.join(&final_filename);
    let mut counter = 1;
    while save_path.exists() {
        let stem = final_filename.trim_end_matches(".mid");
        save_path = album_dir.join(format!("{} ({}).mid", stem, counter));
        counter += 1;
    }

    std::fs::write(&save_path, &data).map_err(|e| format!("Failed to save file: {}", e))?;

    Ok(save_path.to_string_lossy().to_string())
}

// Rename a MIDI file
#[tauri::command]
async fn rename_midi_file(old_path: String, new_name: String) -> Result<String, String> {
    let source = std::path::Path::new(&old_path);

    if !source.exists() {
        return Err("File not found".to_string());
    }

    // Sanitize the new name
    let safe_name: String = new_name
        .chars()
        .filter(|c| !['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(c))
        .collect();

    if safe_name.is_empty() {
        return Err("Invalid filename".to_string());
    }

    // Ensure .mid extension
    let final_name = if safe_name.to_lowercase().ends_with(".mid") {
        safe_name
    } else {
        format!("{}.mid", safe_name)
    };

    // Create new path in same directory
    let parent = source.parent().ok_or("Cannot get parent directory")?;
    let new_path = parent.join(&final_name);

    // Check if target already exists
    if new_path.exists() && new_path != source {
        return Err("A file with that name already exists".to_string());
    }

    std::fs::rename(&source, &new_path)
        .map_err(|e| format!("Failed to rename: {}", e))?;

    Ok(new_path.to_string_lossy().to_string())
}

// Delete a MIDI file
#[tauri::command]
async fn delete_midi_file(path: String) -> Result<(), String> {
    let file_path = std::path::Path::new(&path);

    if !file_path.exists() {
        return Err("File not found".to_string());
    }

    // Verify it's in the album folder for safety
    let album_dir = get_album_folder()?;
    if !file_path.starts_with(&album_dir) {
        return Err("Can only delete files in album folder".to_string());
    }

    std::fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to delete: {}", e))?;

    Ok(())
}

// Open file location in explorer
#[tauri::command]
async fn open_file_location(path: String) -> Result<(), String> {
    let file_path = std::path::Path::new(&path);

    if !file_path.exists() {
        return Err("File not found".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowPosition {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[tauri::command]
async fn get_window_position() -> Result<Option<WindowPosition>, String> {
    let config = load_config();
    if let Some(pos) = config.get("window_position") {
        if let (Some(x), Some(y), Some(w), Some(h)) = (
            pos["x"].as_i64(),
            pos["y"].as_i64(),
            pos["width"].as_u64(),
            pos["height"].as_u64(),
        ) {
            return Ok(Some(WindowPosition {
                x: x as i32,
                y: y as i32,
                width: w as u32,
                height: h as u32,
            }));
        }
    }
    Ok(None)
}

#[tauri::command]
async fn save_window_position(x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
    let mut config = load_config();
    config["window_position"] = serde_json::json!({
        "x": x,
        "y": y,
        "width": width,
        "height": height
    });
    save_config(&config);
    Ok(())
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

    // Get metadata and return file info
    let name = final_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let meta = midi::get_midi_metadata(&final_path.to_string_lossy())
        .unwrap_or(midi::MidiMetadata {
            duration: 0.0, bpm: 120, note_count: 0, note_density: 0.0
        });

    let file_size = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
    let file_hash = compute_file_hash(&final_path).unwrap_or_else(|| format!("{:x}", file_size));

    Ok(MidiFile {
        name,
        path: final_path.to_string_lossy().to_string(),
        duration: meta.duration,
        bpm: meta.bpm,
        note_density: meta.note_density,
        hash: file_hash,
        size: file_size,
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

    app_log!("[UPDATE] Downloading from: {}", download_url);
    app_log!("[UPDATE] Saving to: {:?}", download_path);

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

    app_log!("[UPDATE] Downloaded {} bytes", bytes.len());

    Ok(download_path.to_string_lossy().to_string())
}

// ============ Discovery Server ============

#[tauri::command]
async fn start_discovery_server(port: u16) -> Result<(), String> {
    tokio::spawn(async move {
        if let Err(e) = discovery::start_discovery_server(port).await {
            app_error!("[DISCOVERY] Server error: {}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    if discovery::is_server_running() {
        Ok(())
    } else {
        Err("Failed to start server".to_string())
    }
}

#[tauri::command]
async fn is_discovery_server_running() -> Result<bool, String> {
    Ok(discovery::is_server_running())
}

#[tauri::command]
async fn stop_discovery_server() -> Result<(), String> {
    discovery::stop_discovery_server()
}

#[tauri::command]
async fn load_favorites() -> Result<serde_json::Value, String> {
    let path = get_data_path("favorites.json")?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read favorites: {}", e))?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse favorites: {}", e))?;
        Ok(json)
    } else {
        Ok(serde_json::json!([]))
    }
}

#[tauri::command]
async fn save_favorites(favorites: serde_json::Value) -> Result<(), String> {
    let path = get_data_path("favorites.json")?;
    let content = serde_json::to_string_pretty(&favorites)
        .map_err(|e| format!("Failed to serialize favorites: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write favorites: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn load_playlists() -> Result<serde_json::Value, String> {
    let path = get_data_path("playlists.json")?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read playlists: {}", e))?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse playlists: {}", e))?;
        Ok(json)
    } else {
        Ok(serde_json::json!([]))
    }
}

#[tauri::command]
async fn save_playlists(playlists: serde_json::Value) -> Result<(), String> {
    let path = get_data_path("playlists.json")?;
    let content = serde_json::to_string_pretty(&playlists)
        .map_err(|e| format!("Failed to serialize playlists: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write playlists: {}", e))?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportTrack {
    name: String,
    filename: String,
    duration: f64,
    bpm: u16,
    note_density: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportMetadata {
    export_type: String,
    name: String,
    tracks: Vec<ExportTrack>,
    exported_at: String,
    version: String,
}

// Export favorites to a zip file containing MIDI files and metadata
#[tauri::command]
async fn export_favorites(
    favorites: Vec<serde_json::Value>,
    export_path: String,
) -> Result<(), String> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let file = std::fs::File::create(&export_path)
        .map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut export_tracks = Vec::new();

    for fav in &favorites {
        // Skip favorites without path (not yet hydrated from library)
        let path = match fav["path"].as_str() {
            Some(p) if !p.is_empty() => p,
            _ => {
                app_log!("[EXPORT] Skipping favorite without path: {:?}", fav["name"]);
                continue;
            }
        };
        let name = fav["name"].as_str().unwrap_or("Unknown");
        let duration = fav["duration"].as_f64().unwrap_or(0.0);
        let bpm = fav["bpm"].as_u64().unwrap_or(120) as u16;
        let note_density = fav["note_density"].as_f64().unwrap_or(0.0) as f32;

        let source_path = std::path::Path::new(path);
        if !source_path.exists() {
            app_log!("[EXPORT] File not found, skipping: {}", path);
            continue;
        }

        let filename = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.mid")
            .to_string();

        // Add MIDI file to zip
        let midi_data = std::fs::read(source_path)
            .map_err(|e| format!("Failed to read MIDI file {}: {}", filename, e))?;

        zip.start_file(&filename, options)
            .map_err(|e| format!("Failed to add file to zip: {}", e))?;
        zip.write_all(&midi_data)
            .map_err(|e| format!("Failed to write file data: {}", e))?;

        export_tracks.push(ExportTrack {
            name: name.to_string(),
            filename: filename.clone(),
            duration,
            bpm,
            note_density,
        });

        app_log!("[EXPORT] Added: {} -> {}", name, filename);
    }

    // Add metadata JSON
    let metadata = ExportMetadata {
        export_type: "favorites".to_string(),
        name: "Favorites".to_string(),
        tracks: export_tracks,
        exported_at: chrono_now(),
        version: "1.0".to_string(),
    };

    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    zip.start_file("metadata.json", options)
        .map_err(|e| format!("Failed to add metadata to zip: {}", e))?;
    zip.write_all(metadata_json.as_bytes())
        .map_err(|e| format!("Failed to write metadata: {}", e))?;

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    Ok(())
}

// Export a playlist to a zip file containing MIDI files and metadata
#[tauri::command]
async fn export_playlist(
    playlist_name: String,
    tracks: Vec<serde_json::Value>,
    export_path: String,
) -> Result<(), String> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let file = std::fs::File::create(&export_path)
        .map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut export_tracks = Vec::new();

    for track in &tracks {
        // Skip tracks without path (not yet hydrated from library)
        let path = match track["path"].as_str() {
            Some(p) if !p.is_empty() => p,
            _ => {
                app_log!("[EXPORT] Skipping track without path: {:?}", track["name"]);
                continue;
            }
        };
        let name = track["name"].as_str().unwrap_or("Unknown");
        let duration = track["duration"].as_f64().unwrap_or(0.0);
        let bpm = track["bpm"].as_u64().unwrap_or(120) as u16;
        let note_density = track["note_density"].as_f64().unwrap_or(0.0) as f32;

        let source_path = std::path::Path::new(path);
        if !source_path.exists() {
            app_log!("[EXPORT] File not found, skipping: {}", path);
            continue;
        }

        let filename = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.mid")
            .to_string();

        // Add MIDI file to zip
        let midi_data = std::fs::read(source_path)
            .map_err(|e| format!("Failed to read MIDI file {}: {}", filename, e))?;

        zip.start_file(&filename, options)
            .map_err(|e| format!("Failed to add file to zip: {}", e))?;
        zip.write_all(&midi_data)
            .map_err(|e| format!("Failed to write file data: {}", e))?;

        export_tracks.push(ExportTrack {
            name: name.to_string(),
            filename: filename.clone(),
            duration,
            bpm,
            note_density,
        });

        app_log!("[EXPORT] Added: {} -> {}", name, filename);
    }

    // Add metadata JSON
    let metadata = ExportMetadata {
        export_type: "playlist".to_string(),
        name: playlist_name,
        tracks: export_tracks,
        exported_at: chrono_now(),
        version: "1.0".to_string(),
    };

    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    zip.start_file("metadata.json", options)
        .map_err(|e| format!("Failed to add metadata to zip: {}", e))?;
    zip.write_all(metadata_json.as_bytes())
        .map_err(|e| format!("Failed to write metadata: {}", e))?;

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    Ok(())
}

// Import result structure
#[derive(Debug, Serialize, Deserialize)]
struct ImportResult {
    imported_files: Vec<MidiFile>,
    export_type: String,
    name: String,
}

// Compute hash from bytes in memory (matches compute_file_hash logic)
fn compute_hash_from_bytes(data: &[u8]) -> String {
    let file_size = data.len() as u64;
    let bytes_to_read = std::cmp::min(8192, data.len());

    let mut hash: u64 = file_size;
    for byte in &data[..bytes_to_read] {
        hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
    }

    format!("{:016x}", hash)
}

// Build a map of hash -> MidiFile for existing files in album
fn get_existing_files_by_hash(album_dir: &std::path::Path) -> std::collections::HashMap<String, MidiFile> {
    let mut map = std::collections::HashMap::new();

    if let Ok(entries) = std::fs::read_dir(album_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mid") {
                if let Some(hash) = compute_file_hash(&path) {
                    let name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let meta = midi::get_midi_metadata(&path.to_string_lossy())
                        .unwrap_or(midi::MidiMetadata {
                            duration: 0.0, bpm: 120, note_count: 0, note_density: 0.0
                        });

                    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

                    map.insert(hash.clone(), MidiFile {
                        name,
                        path: path.to_string_lossy().to_string(),
                        duration: meta.duration,
                        bpm: meta.bpm,
                        note_density: meta.note_density,
                        hash,
                        size: file_size,
                    });
                }
            }
        }
    }

    map
}

// Import a zip file containing MIDI files (from exported favorites/playlist)
#[tauri::command]
async fn import_zip(zip_path: String) -> Result<ImportResult, String> {
    use std::io::Read;

    let file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("Failed to open zip file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;

    let album_dir = get_album_folder()?;

    // Create album folder if it doesn't exist
    if !album_dir.exists() {
        std::fs::create_dir_all(&album_dir)
            .map_err(|e| format!("Failed to create album folder: {}", e))?;
    }

    // Get existing files by hash to skip duplicates
    let existing_files = get_existing_files_by_hash(&album_dir);

    let mut imported_files = Vec::new();
    let mut export_type = "unknown".to_string();
    let mut export_name = "Import".to_string();

    // First pass: read metadata if exists
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if file.name() == "metadata.json" {
            let mut contents = String::new();
            file.read_to_string(&mut contents).ok();
            if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&contents) {
                export_type = meta["export_type"].as_str().unwrap_or("unknown").to_string();
                export_name = meta["name"].as_str().unwrap_or("Import").to_string();
            }
            break;
        }
    }

    // Re-open archive for extraction (can't reuse after iteration)
    let file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("Failed to reopen zip file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;

    // Second pass: extract MIDI files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let filename = file.name().to_string();

        // Skip metadata and non-MIDI files
        if filename == "metadata.json" || !filename.to_lowercase().ends_with(".mid") {
            continue;
        }

        // Read file contents
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| format!("Failed to read {}: {}", filename, e))?;

        // Verify it's a valid MIDI file
        if contents.len() < 4 || &contents[0..4] != b"MThd" {
            app_log!("[IMPORT] Skipping invalid MIDI: {}", filename);
            continue;
        }

        // Compute hash to check if file already exists
        let file_hash = compute_hash_from_bytes(&contents);

        // Check if file with same hash already exists
        if let Some(existing) = existing_files.get(&file_hash).cloned() {
            app_log!("[IMPORT] Skipping duplicate (hash exists): {} -> {}", filename, existing.path);
            imported_files.push(existing);
            continue;
        }

        // Determine save path (handle filename duplicates)
        let mut save_path = album_dir.join(&filename);
        let mut counter = 1;
        while save_path.exists() {
            let stem = filename.trim_end_matches(".mid");
            save_path = album_dir.join(format!("{} ({}).mid", stem, counter));
            counter += 1;
        }

        // Write file
        std::fs::write(&save_path, &contents)
            .map_err(|e| format!("Failed to save {}: {}", filename, e))?;

        // Get metadata for the imported file
        let name = save_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let meta = midi::get_midi_metadata(&save_path.to_string_lossy())
            .unwrap_or(midi::MidiMetadata {
                duration: 0.0, bpm: 120, note_count: 0, note_density: 0.0
            });

        let file_size = contents.len() as u64;

        imported_files.push(MidiFile {
            name,
            path: save_path.to_string_lossy().to_string(),
            duration: meta.duration,
            bpm: meta.bpm,
            note_density: meta.note_density,
            hash: file_hash,
            size: file_size,
        });

        app_log!("[IMPORT] Imported: {}", save_path.to_string_lossy());
    }

    Ok(ImportResult {
        imported_files,
        export_type,
        name: export_name,
    })
}

// Helper to get current timestamp (simple implementation without chrono crate)
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Convert to simple ISO-like format
    let days = secs / 86400;
    let years_since_1970 = days / 365;
    let year = 1970 + years_since_1970;
    let remaining_days = days % 365;
    let month = (remaining_days / 30) + 1;
    let day = (remaining_days % 30) + 1;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hours, minutes, seconds)
}

#[tauri::command]
async fn install_update(zip_path: String, app_handle: AppHandle) -> Result<(), String> {
    app_log!("[UPDATE] Installing from: {}", zip_path);

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

    app_log!("[UPDATE] Created update script at: {:?}", script_path);

    // Start the update script
    std::process::Command::new("cmd")
        .args(["/C", "start", "", "/MIN", script_path.to_str().unwrap()])
        .spawn()
        .map_err(|e| format!("Failed to start update script: {}", e))?;

    // Exit the app
    app_log!("[UPDATE] Exiting for update...");
    app_handle.exit(0);

    Ok(())
}

fn register_global_hotkeys() -> Vec<(String, bool)> {
    let mut results = Vec::new();
    let kb = get_keybindings();

    unsafe {
        // Pause/Resume
        if let Some(vk) = key_to_vk(&kb.pause_resume) {
            let result = RegisterHotKey(None, HOTKEY_PAUSE_RESUME, MOD_NOREPEAT, vk);
            results.push((format!("{} (Pause/Resume)", kb.pause_resume), result.is_ok()));
        }

        // Stop - also register End as backup
        if let Some(vk) = key_to_vk(&kb.stop) {
            let result = RegisterHotKey(None, HOTKEY_STOP_F12, MOD_NOREPEAT, vk);
            results.push((format!("{} (Stop)", kb.stop), result.is_ok()));
        }
        let result = RegisterHotKey(None, HOTKEY_STOP_END, MOD_NOREPEAT, VK_END.0 as u32);
        results.push(("End (Stop backup)".to_string(), result.is_ok()));

        // Previous
        if let Some(vk) = key_to_vk(&kb.previous) {
            let result = RegisterHotKey(None, HOTKEY_PREV_F10, MOD_NOREPEAT, vk);
            results.push((format!("{} (Previous)", kb.previous), result.is_ok()));
        }

        // Next
        if let Some(vk) = key_to_vk(&kb.next) {
            let result = RegisterHotKey(None, HOTKEY_NEXT_F11, MOD_NOREPEAT, vk);
            results.push((format!("{} (Next)", kb.next), result.is_ok()));
        }
    }

    results
}

// Cached keybinding VK codes for low-level hook
static mut CACHED_PAUSE_RESUME_VK: u32 = 0x78; // F9
static mut CACHED_STOP_VK: u32 = 0x7B;         // F12
static mut CACHED_PREVIOUS_VK: u32 = 0x79;     // F10
static mut CACHED_NEXT_VK: u32 = 0x7A;         // F11
static mut CACHED_MODE_PREV_VK: u32 = 0xDB;    // [
static mut CACHED_MODE_NEXT_VK: u32 = 0xDD;    // ]
static mut CACHED_TOGGLE_MINI_VK: u32 = 0x2D;  // Insert
static mut KEYBINDINGS_DISABLED: bool = false; // Disable during recording
static mut RECORDING_MODE: bool = false;       // When true, emit key names instead of actions

// Convert VK code to key name string
fn vk_to_key(vk: u32) -> Option<String> {
    match vk {
        0x1B => Some("Escape".into()),
        0x70 => Some("F1".into()), 0x71 => Some("F2".into()), 0x72 => Some("F3".into()), 0x73 => Some("F4".into()),
        0x74 => Some("F5".into()), 0x75 => Some("F6".into()), 0x76 => Some("F7".into()), 0x77 => Some("F8".into()),
        0x78 => Some("F9".into()), 0x79 => Some("F10".into()), 0x7A => Some("F11".into()), 0x7B => Some("F12".into()),
        0x2D => Some("Insert".into()), 0x2E => Some("Delete".into()),
        0x24 => Some("Home".into()), 0x23 => Some("End".into()),
        0x21 => Some("PageUp".into()), 0x22 => Some("PageDown".into()),
        0x91 => Some("ScrollLock".into()), 0x13 => Some("Pause".into()), 0x90 => Some("NumLock".into()),
        0x2C => Some("PrintScreen".into()),
        0x26 => Some("Up".into()), 0x28 => Some("Down".into()), 0x25 => Some("Left".into()), 0x27 => Some("Right".into()),
        0xDB => Some("[".into()), 0xDD => Some("]".into()), 0xC0 => Some("`".into()),
        0xBD => Some("-".into()), 0xBB => Some("=".into()), 0xDC => Some("\\".into()),
        0xBA => Some(";".into()), 0xDE => Some("'".into()),
        0xBC => Some(",".into()), 0xBE => Some(".".into()), 0xBF => Some("/".into()),
        // Letters A-Z
        0x41..=0x5A => Some(((b'A' + (vk - 0x41) as u8) as char).to_string()),
        // Numbers 0-9
        0x30..=0x39 => Some(((b'0' + (vk - 0x30) as u8) as char).to_string()),
        // Numpad
        0x60..=0x69 => Some(format!("Numpad{}", vk - 0x60)),
        _ => None,
    }
}

fn cache_keybinding_vks() {
    let kb = get_keybindings();
    unsafe {
        CACHED_PAUSE_RESUME_VK = key_to_vk(&kb.pause_resume).unwrap_or(0x78);
        CACHED_STOP_VK = key_to_vk(&kb.stop).unwrap_or(0x7B);
        CACHED_PREVIOUS_VK = key_to_vk(&kb.previous).unwrap_or(0x79);
        CACHED_NEXT_VK = key_to_vk(&kb.next).unwrap_or(0x7A);
        CACHED_MODE_PREV_VK = key_to_vk(&kb.mode_prev).unwrap_or(0xDB);
        CACHED_MODE_NEXT_VK = key_to_vk(&kb.mode_next).unwrap_or(0xDD);
        CACHED_TOGGLE_MINI_VK = key_to_vk(&kb.toggle_mini).unwrap_or(0x2D);
    }
    app_log!("[KEYBINDINGS] Reloaded: pause={:02X} stop={:02X} prev={:02X} next={:02X}",
        unsafe { CACHED_PAUSE_RESUME_VK }, unsafe { CACHED_STOP_VK },
        unsafe { CACHED_PREVIOUS_VK }, unsafe { CACHED_NEXT_VK });
}

// Low-level keyboard hook callback for all keybindings
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
                let vk = kb_struct.vkCode;

                // Recording mode: emit key name for binding capture
                if RECORDING_MODE {
                    // Skip modifier keys
                    if vk != 0x10 && vk != 0x11 && vk != 0x12 && vk != 0xA0 && vk != 0xA1 && vk != 0xA2 && vk != 0xA3 && vk != 0xA4 && vk != 0xA5 && vk != 0x5B && vk != 0x5C {
                        if let Some(key_name) = vk_to_key(vk) {
                            let _ = app_handle.emit("key-captured", key_name);
                        }
                    }
                }
                // Normal mode: emit actions
                else if !KEYBINDINGS_DISABLED {
                    if vk == CACHED_PAUSE_RESUME_VK {
                        let _ = app_handle.emit("global-shortcut", "pause_resume");
                    } else if vk == CACHED_STOP_VK || vk == VK_END.0 as u32 {
                        let _ = app_handle.emit("global-shortcut", "stop");
                    } else if vk == CACHED_PREVIOUS_VK {
                        let _ = app_handle.emit("global-shortcut", "previous");
                    } else if vk == CACHED_NEXT_VK {
                        let _ = app_handle.emit("global-shortcut", "next");
                    } else if vk == CACHED_MODE_PREV_VK {
                        let _ = app_handle.emit("global-shortcut", "mode_prev");
                    } else if vk == CACHED_MODE_NEXT_VK {
                        let _ = app_handle.emit("global-shortcut", "mode_next");
                    } else if vk == CACHED_TOGGLE_MINI_VK {
                        let _ = app_handle.emit("global-shortcut", "toggle_mini");
                    }
                }
            }
        }
    }

    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

fn start_hotkey_listener(app_handle: AppHandle) {
    // Cache keybinding VK codes for low-level hook
    cache_keybinding_vks();

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
                app_error!("Failed to install low-level keyboard hook for F12");
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
                    app_error!("GetMessageW error");
                    break;
                }
                if result.0 == 0 {
                    // WM_QUIT received
                    break;
                }

                if msg.message == WM_HOTKEY && !KEYBINDINGS_DISABLED {
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
            app_error!("Failed to set process priority to HIGH");
        }
    }
}

fn main() {
    // Initialize logging first
    init_logger();

    // Set high priority for accurate MIDI timing
    set_high_priority();

    // Load saved settings from config
    load_saved_album_path();
    load_saved_note_keys();
    load_custom_window_keywords();
    load_saved_keybindings();

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
            get_midi_tracks,
            play_midi,
            play_midi_band,
            pause_resume,
            stop_playback,
            get_playback_status,
            set_loop_mode,
            set_note_mode,
            get_note_mode,
            set_track_filter,
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
            set_note_keys,
            get_note_keys,
            reset_note_keys,
            set_custom_window_keywords,
            get_custom_window_keywords,
            cmd_get_keybindings,
            cmd_set_keybindings,
            cmd_reset_keybindings,
            cmd_set_keybindings_enabled,
            cmd_unfocus_window,
            press_key,
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
            read_midi_base64,
            check_midi_exists,
            save_temp_midi,
            verify_midi_data,
            save_midi_from_base64,
            rename_midi_file,
            delete_midi_file,
            open_file_location,
            get_window_position,
            save_window_position,
            check_for_update,
            download_update,
            install_update,
            start_discovery_server,
            is_discovery_server_running,
            stop_discovery_server,
            load_favorites,
            save_favorites,
            load_playlists,
            save_playlists,
            export_favorites,
            export_playlist,
            import_zip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
