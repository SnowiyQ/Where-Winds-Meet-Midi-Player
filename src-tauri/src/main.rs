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
                println!("Loaded album path: {}", path);
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

fn load_saved_qwertz_mode() {
    let config = load_config();
    if let Some(enabled) = config["qwertz_mode"].as_bool() {
        keyboard::set_qwertz_mode(enabled);
        println!("Loaded QWERTZ mode: {}", enabled);
    }
}

fn save_qwertz_mode(enabled: bool) {
    let mut config = load_config();
    config["qwertz_mode"] = serde_json::json!(enabled);
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
        println!("Loaded custom window keywords");
    }
}

fn save_custom_window_keywords(keywords: &[String]) {
    let mut config = load_config();
    config["custom_window_keywords"] = serde_json::json!(keywords);
    save_config(&config);
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

#[derive(Debug, Serialize, Deserialize)]
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
async fn set_qwertz_mode(enabled: bool) -> Result<(), String> {
    keyboard::set_qwertz_mode(enabled);
    save_qwertz_mode(enabled);
    Ok(())
}

#[tauri::command]
async fn get_qwertz_mode() -> Result<bool, String> {
    Ok(keyboard::get_qwertz_mode())
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

// ============ Discovery Server ============

#[tauri::command]
async fn start_discovery_server(port: u16) -> Result<(), String> {
    tokio::spawn(async move {
        if let Err(e) = discovery::start_discovery_server(port).await {
            eprintln!("[DISCOVERY] Server error: {}", e);
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

    // Load saved settings from config
    load_saved_album_path();
    load_saved_qwertz_mode();
    load_custom_window_keywords();

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
            set_qwertz_mode,
            get_qwertz_mode,
            set_custom_window_keywords,
            get_custom_window_keywords,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
