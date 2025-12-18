//! Real-time MIDI device input handling
//!
//! Supports MIDI 1.0 standard devices:
//! - USB-MIDI controllers
//! - 5-pin DIN MIDI via USB adapters
//! - Bluetooth MIDI (if OS exposes as standard MIDI port)
//! - Virtual MIDI ports (loopMIDI, IAC Driver, etc.)

use midir::{MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

use crate::keyboard;
use crate::midi::{KeyMode, NoteMode};

/// Connection state for the frontend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MidiConnectionState {
    NoDevices,
    DevicesAvailable,
    Connecting,
    Connected,
    Listening,
    Disconnected,
    Error,
}

/// Live note event sent to frontend for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveNoteEvent {
    pub midi_note: u8,
    pub key: String,
    pub note_name: String,
    pub velocity: u8,
}

/// MIDI Input Manager state
pub struct MidiInputState {
    /// Active MIDI connection (if any)
    connection: Option<MidiInputConnection<()>>,
    /// List of available port names
    available_ports: Vec<String>,
    /// Currently selected port index
    selected_port: Option<usize>,
    /// Connection state
    state: MidiConnectionState,
}

impl Default for MidiInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiInputState {
    pub fn new() -> Self {
        MidiInputState {
            connection: None,
            available_ports: Vec::new(),
            selected_port: None,
            state: MidiConnectionState::NoDevices,
        }
    }

    pub fn get_state(&self) -> MidiConnectionState {
        self.state
    }

    #[allow(dead_code)]
    pub fn get_available_ports(&self) -> &[String] {
        &self.available_ports
    }

    #[allow(dead_code)]
    pub fn get_selected_port(&self) -> Option<usize> {
        self.selected_port
    }
}

/// Refresh and list available MIDI input devices
pub fn list_midi_devices(midi_state: &mut MidiInputState) -> Vec<String> {
    let mut ports = Vec::new();

    match MidiInput::new("WWM Overlay Scanner") {
        Ok(midi_in) => {
            let in_ports = midi_in.ports();
            for port in in_ports.iter() {
                if let Ok(name) = midi_in.port_name(port) {
                    ports.push(name);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to create MIDI input for scanning: {}", e);
        }
    }

    midi_state.available_ports = ports.clone();
    midi_state.state = if ports.is_empty() {
        MidiConnectionState::NoDevices
    } else {
        MidiConnectionState::DevicesAvailable
    };

    ports
}

/// Start listening to a MIDI device
pub fn start_listening(
    midi_state: Arc<Mutex<MidiInputState>>,
    device_index: usize,
    app_handle: AppHandle,
    note_mode: Arc<AtomicU8>,
    key_mode: Arc<AtomicU8>,
    octave_shift: Arc<AtomicI8>,
    transpose: Arc<AtomicI8>,
    is_listening: Arc<AtomicBool>,
) -> Result<String, String> {
    let mut state = midi_state
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    // Stop any existing connection
    if state.connection.is_some() {
        state.connection = None;
    }

    if device_index >= state.available_ports.len() {
        return Err("Invalid device index".to_string());
    }

    let device_name = state.available_ports[device_index].clone();
    state.state = MidiConnectionState::Connecting;
    state.selected_port = Some(device_index);

    // Drop the lock before creating the connection
    drop(state);

    // Create new MIDI input
    let midi_in = MidiInput::new("WWM Overlay Live")
        .map_err(|e| format!("Failed to create MIDI input: {}", e))?;

    let ports = midi_in.ports();
    if device_index >= ports.len() {
        return Err("Device no longer available".to_string());
    }

    let port = &ports[device_index];

    // Clone values for the callback
    let app_handle_clone = app_handle.clone();
    let note_mode_clone = note_mode.clone();
    let key_mode_clone = key_mode.clone();
    let octave_shift_clone = octave_shift.clone();
    let transpose_clone = transpose.clone();
    let is_listening_clone = is_listening.clone();
    let _midi_state_clone = midi_state.clone();

    // Create the connection with callback
    let connection = midi_in
        .connect(
            port,
            "wwm-live-input",
            move |_timestamp, message, _| {
                if !is_listening_clone.load(Ordering::SeqCst) {
                    return;
                }

                handle_midi_message(
                    message,
                    &app_handle_clone,
                    &note_mode_clone,
                    &key_mode_clone,
                    &octave_shift_clone,
                    &transpose_clone,
                );
            },
            (),
        )
        .map_err(|e| format!("Failed to connect to MIDI device: {}", e))?;

    // Update state
    let mut state = midi_state
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    state.connection = Some(connection);
    state.state = MidiConnectionState::Connected;
    is_listening.store(true, Ordering::SeqCst);

    // Emit connection event
    let _ = app_handle.emit("midi-device-connected", &device_name);

    Ok(device_name)
}

/// Stop listening to MIDI device
pub fn stop_listening(
    midi_state: Arc<Mutex<MidiInputState>>,
    is_listening: Arc<AtomicBool>,
    app_handle: &AppHandle,
) -> Result<(), String> {
    is_listening.store(false, Ordering::SeqCst);

    let mut state = midi_state
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    // Drop the connection (this closes it)
    state.connection = None;
    state.state = if state.available_ports.is_empty() {
        MidiConnectionState::NoDevices
    } else {
        MidiConnectionState::DevicesAvailable
    };

    let _ = app_handle.emit("midi-device-disconnected", ());

    Ok(())
}

/// Handle incoming MIDI message
fn handle_midi_message(
    message: &[u8],
    app_handle: &AppHandle,
    _note_mode: &Arc<AtomicU8>,
    key_mode: &Arc<AtomicU8>,
    octave_shift: &Arc<AtomicI8>,
    transpose: &Arc<AtomicI8>,
) {
    if message.len() < 3 {
        return;
    }

    let status = message[0];
    let note = message[1];
    let velocity = message[2];

    // Check for Note On (0x90-0x9F) with velocity > 0
    // Note: Note On with velocity 0 is treated as Note Off per MIDI spec
    let is_note_on = (status & 0xF0) == 0x90 && velocity > 0;

    // Check for Note Off (0x80-0x8F) or Note On with velocity 0
    let _is_note_off = (status & 0xF0) == 0x80 || ((status & 0xF0) == 0x90 && velocity == 0);

    if is_note_on {
        // Get current settings
        let current_key_mode = KeyMode::from(key_mode.load(Ordering::SeqCst));
        let current_octave_shift = octave_shift.load(Ordering::SeqCst) as i32;
        let current_transpose = transpose.load(Ordering::SeqCst) as i32;

        // Calculate total transpose (octave shift only)
        let total_transpose = current_transpose + (current_octave_shift * 12);

        // Live input: use direct chromatic mapping (Closest mode), bypass note_mode
        let key = map_note_to_key(
            note as i32,
            total_transpose,
            NoteMode::Closest,
            current_key_mode,
        );

        // Press the key
        keyboard::key_down(&key);

        // Small delay then release (game uses tap, not hold)
        std::thread::spawn({
            let key = key.clone();
            move || {
                std::thread::sleep(std::time::Duration::from_millis(30));
                keyboard::key_up(&key);
            }
        });

        // Emit event for frontend visualization
        let note_name = midi_note_to_name(note);
        let event = LiveNoteEvent {
            midi_note: note,
            key: key.clone(),
            note_name,
            velocity,
        };
        let _ = app_handle.emit("live-note-event", &event);
    }

    // Note: We don't need to handle note_off explicitly since we auto-release after 30ms
}

/// Map MIDI note to game key (same logic as midi.rs)
pub fn map_note_to_key(
    note: i32,
    transpose: i32,
    note_mode: NoteMode,
    key_mode: KeyMode,
) -> String {
    match key_mode {
        KeyMode::Keys36 => match note_mode {
            NoteMode::Closest => note_to_key_36_closest(note, transpose),
            NoteMode::Quantize => note_to_key_36_quantize(note, transpose),
            NoteMode::TransposeOnly => note_to_key_36_transpose(note, transpose),
            NoteMode::Pentatonic => note_to_key_36_pentatonic(note, transpose),
            NoteMode::Chromatic => note_to_key_36_chromatic(note, transpose),
            NoteMode::Raw => note_to_key_36_raw(note),
            NoteMode::Python => note_to_key_python(note, transpose),
            NoteMode::Wide => note_to_key_36_wide(note, transpose),
            NoteMode::Sharps => note_to_key_36_sharps(note, transpose),
        },
        KeyMode::Keys21 => {
            match note_mode {
                NoteMode::Closest => note_to_key(note, transpose),
                NoteMode::Quantize => note_to_key_quantize(note, transpose),
                NoteMode::TransposeOnly => note_to_key_transpose(note, transpose),
                NoteMode::Pentatonic => note_to_key_pentatonic(note, transpose),
                NoteMode::Chromatic => note_to_key_chromatic(note, transpose),
                NoteMode::Raw => note_to_key_raw(note),
                NoteMode::Python => note_to_key_python(note, transpose),
                NoteMode::Wide => note_to_key_wide(note, transpose),
                NoteMode::Sharps => note_to_key(note, transpose), // Falls back to Closest in 21-key
            }
        }
    }
}

/// Convert MIDI note number to note name (e.g., 60 -> "C4")
fn midi_note_to_name(note: u8) -> String {
    const NOTE_NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (note as i32 / 12) - 1;
    let note_idx = (note % 12) as usize;
    format!("{}{}", NOTE_NAMES[note_idx], octave)
}

// ============================================================================
// Note mapping functions (copied from midi.rs to avoid circular dependencies)
// These match the exact logic in midi.rs
// ============================================================================

// 21-key mode: Basic keys for 3 octaves (7 notes each)
const LOW_KEYS: [&str; 7] = ["z", "x", "c", "v", "b", "n", "m"];
const MID_KEYS: [&str; 7] = ["a", "s", "d", "f", "g", "h", "j"];
const HIGH_KEYS: [&str; 7] = ["q", "w", "e", "r", "t", "y", "u"];

const INSTRUMENT_NOTES: [i32; 21] = [
    48, 50, 52, 53, 55, 57, 59, // LOW_SCALE
    60, 62, 64, 65, 67, 69, 71, // MID_SCALE
    72, 74, 76, 77, 79, 81, 83, // HIGH_SCALE
];

/// Normalize note into the instrument range (48-83)
fn normalize_into_range(note: i32) -> i32 {
    let lo = INSTRUMENT_NOTES[0]; // 48
    let hi = INSTRUMENT_NOTES[20]; // 83

    let mut target = note;
    while target < lo {
        target += 12;
    }
    while target > hi {
        target -= 12;
    }
    target
}

fn note_to_key(note: i32, transpose: i32) -> String {
    let target = normalize_into_range(note + transpose);

    let mut best_idx: usize = 0;
    let mut best_dist = (INSTRUMENT_NOTES[0] - target).abs();

    for (i, &inst_note) in INSTRUMENT_NOTES.iter().enumerate() {
        let dist = (inst_note - target).abs();
        if dist < best_dist {
            best_idx = i;
            best_dist = dist;
        }
    }

    let all_keys = [
        LOW_KEYS.as_slice(),
        MID_KEYS.as_slice(),
        HIGH_KEYS.as_slice(),
    ]
    .concat();
    all_keys[best_idx].to_string()
}

fn note_to_key_quantize(note: i32, transpose: i32) -> String {
    note_to_key(note, transpose)
}

fn note_to_key_transpose(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;

    let key_idx = match semitone {
        0 => 0,
        1 => 0,
        2 => 1,
        3 => 1,
        4 => 2,
        5 => 3,
        6 => 3,
        7 => 4,
        8 => 4,
        9 => 5,
        10 => 5,
        11 => 6,
        _ => 0,
    };

    let octave = ((target - 48) / 12).clamp(0, 2) as usize;
    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

fn note_to_key_pentatonic(note: i32, transpose: i32) -> String {
    let normalized = normalize_into_range(note + transpose);
    let semitone = ((normalized - 48) % 12 + 12) % 12;
    let octave = ((normalized - 48) / 12).clamp(0, 2) as usize;

    let key_idx = match semitone {
        0 | 1 => 0,
        2 | 3 | 4 => 1,
        5 | 6 | 7 => 2,
        8 | 9 => 3,
        10 | 11 => 4,
        _ => 0,
    };

    let penta_keys = [0, 1, 2, 4, 5];
    let actual_idx = penta_keys[key_idx.min(4)];

    match octave {
        0 => LOW_KEYS[actual_idx].to_string(),
        1 => MID_KEYS[actual_idx].to_string(),
        _ => HIGH_KEYS[actual_idx].to_string(),
    }
}

fn note_to_key_chromatic(note: i32, transpose: i32) -> String {
    let normalized = normalize_into_range(note + transpose);
    let semitone = ((normalized - 48) % 12 + 12) % 12;
    let octave = ((normalized - 48) / 12).clamp(0, 2) as usize;

    let key_idx = match semitone {
        0 => 0,
        1 => 0,
        2 => 1,
        3 => 2,
        4 => 2,
        5 => 3,
        6 => 3,
        7 => 4,
        8 => 4,
        9 => 5,
        10 => 6,
        11 => 6,
        _ => 0,
    };

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

fn note_to_key_raw(note: i32) -> String {
    let key_idx = ((note % 21) + 21) % 21;
    let all_keys = [
        LOW_KEYS.as_slice(),
        MID_KEYS.as_slice(),
        HIGH_KEYS.as_slice(),
    ]
    .concat();
    all_keys[key_idx as usize].to_string()
}

fn note_to_key_python(note: i32, transpose: i32) -> String {
    const PY_INSTRUMENT_NOTES: [i32; 21] = [
        48, 50, 52, 53, 55, 57, 59, 60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83,
    ];

    const PY_KEYS: [&str; 21] = [
        "z", "x", "c", "v", "b", "n", "m", "a", "s", "d", "f", "g", "h", "j", "q", "w", "e", "r",
        "t", "y", "u",
    ];

    let lo = PY_INSTRUMENT_NOTES[0];
    let hi = PY_INSTRUMENT_NOTES[20];

    let mut target = note + transpose;
    while target < lo {
        target += 12;
    }
    while target > hi {
        target -= 12;
    }

    let mut best_idx: usize = 0;
    let mut best_dist = (PY_INSTRUMENT_NOTES[0] - target).abs();

    for (i, &inst_note) in PY_INSTRUMENT_NOTES.iter().enumerate() {
        let dist = (inst_note - target).abs();
        if dist < best_dist {
            best_idx = i;
            best_dist = dist;
        }
    }

    PY_KEYS[best_idx].to_string()
}

fn note_to_key_wide(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let key_idx = ((target - 36) * 21 / 60).clamp(0, 20) as usize;
    let all_keys = [
        LOW_KEYS.as_slice(),
        MID_KEYS.as_slice(),
        HIGH_KEYS.as_slice(),
    ]
    .concat();
    all_keys[key_idx].to_string()
}

// 36-key mode functions
fn get_octave_36(target: i32) -> usize {
    if target < 60 {
        0
    } else if target < 72 {
        1
    } else {
        2
    }
}

fn semitone_to_key_36(semitone: i32, octave: usize) -> String {
    match semitone {
        0 => match octave {
            0 => "z",
            1 => "a",
            _ => "q",
        }
        .to_string(),
        2 => match octave {
            0 => "x",
            1 => "s",
            _ => "w",
        }
        .to_string(),
        4 => match octave {
            0 => "c",
            1 => "d",
            _ => "e",
        }
        .to_string(),
        5 => match octave {
            0 => "v",
            1 => "f",
            _ => "r",
        }
        .to_string(),
        7 => match octave {
            0 => "b",
            1 => "g",
            _ => "t",
        }
        .to_string(),
        9 => match octave {
            0 => "n",
            1 => "h",
            _ => "y",
        }
        .to_string(),
        11 => match octave {
            0 => "m",
            1 => "j",
            _ => "u",
        }
        .to_string(),
        1 => match octave {
            0 => "shift+z",
            1 => "shift+a",
            _ => "shift+q",
        }
        .to_string(),
        3 => match octave {
            0 => "ctrl+c",
            1 => "ctrl+d",
            _ => "ctrl+e",
        }
        .to_string(),
        6 => match octave {
            0 => "shift+v",
            1 => "shift+f",
            _ => "shift+r",
        }
        .to_string(),
        8 => match octave {
            0 => "shift+b",
            1 => "shift+g",
            _ => "shift+t",
        }
        .to_string(),
        10 => match octave {
            0 => "ctrl+m",
            1 => "ctrl+j",
            _ => "ctrl+u",
        }
        .to_string(),
        _ => "a".to_string(),
    }
}

fn note_to_key_36_closest(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);
    semitone_to_key_36(semitone, octave)
}

fn note_to_key_36_quantize(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);

    let quantized = match semitone {
        0 | 1 => 0,
        2 | 3 => 2,
        4 => 4,
        5 | 6 => 5,
        7 | 8 => 7,
        9 | 10 => 9,
        11 => 11,
        _ => 0,
    };

    semitone_to_key_36(quantized, octave)
}

fn note_to_key_36_transpose(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);
    semitone_to_key_36(semitone, octave)
}

fn note_to_key_36_pentatonic(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);

    let penta = match semitone {
        0 | 1 => 0,
        2 | 3 => 2,
        4 | 5 | 6 => 4,
        7 | 8 => 7,
        9 | 10 | 11 => 9,
        _ => 0,
    };

    semitone_to_key_36(penta, octave)
}

fn note_to_key_36_chromatic(note: i32, transpose: i32) -> String {
    note_to_key_36_closest(note, transpose)
}

fn note_to_key_36_raw(note: i32) -> String {
    let key_idx = ((note % 36) + 36) % 36;
    let octave = (key_idx / 12) as usize;
    let semitone = key_idx % 12;
    semitone_to_key_36(semitone, octave)
}

fn note_to_key_36_wide(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = if target < 54 {
        0
    } else if target < 66 {
        1
    } else {
        2
    };
    semitone_to_key_36(semitone, octave)
}

fn note_to_key_36_sharps(note: i32, transpose: i32) -> String {
    let target = note + transpose + 1;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);
    semitone_to_key_36(semitone, octave)
}
