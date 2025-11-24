#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use tauri::{State, Window};
use tauri_plugin_global_shortcut::Builder as GlobalShortcutPlugin;
use serde::{Serialize, Deserialize};

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

#[derive(Debug, Serialize, Deserialize)]
struct PlaylistItem {
    midi_file: MidiFile,
    order: usize,
}

// Load MIDI files from album folder
#[tauri::command]
async fn load_midi_files() -> Result<Vec<MidiFile>, String> {
    // Get the directory where the executable is located
    let exe_path = std::env::current_exe()
        .map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent()
        .ok_or("Failed to get executable directory")?;
    let album_path = exe_dir.join("album");

    let mut files = Vec::new();

    if album_path.exists() {
        let entries = std::fs::read_dir(album_path)
            .map_err(|e| e.to_string())?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("mid") {
                    let name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    // Don't load MIDI file here - just list files quickly
                    files.push(MidiFile {
                        name,
                        path: path.to_string_lossy().to_string(),
                        duration: 0.0, // Duration will be loaded when playing
                    });
                }
            }
        }
    }

    Ok(files)
}

// Play MIDI file
#[tauri::command]
async fn play_midi(
    path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    window: Window
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();

    // Stop current playback if any
    app_state.stop_playback();

    // Load and play new MIDI file
    app_state.load_midi(&path)?;
    app_state.start_playback(window)?;

    // Release the lock before focusing
    drop(app_state);

    // Focus game window after starting playback
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = keyboard::focus_black_desert_window();

    Ok(())
}

// Pause/Resume playback
#[tauri::command]
async fn pause_resume(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<PlaybackState, String> {
    let mut app_state = state.lock().unwrap();
    app_state.toggle_pause();
    Ok(app_state.get_playback_state())
}

// Stop playback
#[tauri::command]
async fn stop_playback(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.stop_playback();
    Ok(())
}

// Get current playback status
#[tauri::command]
async fn get_playback_status(
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<PlaybackState, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.get_playback_state())
}

// Toggle loop mode
#[tauri::command]
async fn set_loop_mode(
    enabled: bool,
    state: State<'_, Arc<Mutex<AppState>>>
) -> Result<(), String> {
    let mut app_state = state.lock().unwrap();
    app_state.set_loop_mode(enabled);
    Ok(())
}

// Check if game is focused
#[tauri::command]
async fn is_game_focused() -> Result<bool, String> {
    keyboard::is_black_desert_focused()
        .map_err(|e| e.to_string())
}

// Toggle window interaction mode (draggable vs click-through)
#[tauri::command]
async fn set_interaction_mode(window: Window, interactive: bool) -> Result<(), String> {
    window.set_ignore_cursor_events(!interactive)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Focus Black Desert window
#[tauri::command]
async fn focus_game_window() -> Result<(), String> {
    keyboard::focus_black_desert_window()
        .map_err(|e| e.to_string())
}

// Seek to position
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

fn main() {
    let app_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(GlobalShortcutPlugin::default().build())
        .manage(app_state)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
