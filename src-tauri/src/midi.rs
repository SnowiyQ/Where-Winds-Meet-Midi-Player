use midly::{MidiMessage, Smf, TrackEventKind};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Window};

/// Note calculation mode - how MIDI notes are mapped to game keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NoteMode {
    Closest = 0,       // Find closest available note (original behavior)
    Quantize = 1,      // Snap to exact scale notes only
    TransposeOnly = 2, // Just shift octaves, direct mapping
    Pentatonic = 3,    // Map to pentatonic scale (5 notes)
    Chromatic = 4,     // Detailed chromatic mapping
    Raw = 5,           // Raw 1:1 mapping, no transpose
    Python = 6,        // Exact 1:1 copy of Python main.py logic
    Wide = 7,          // Spread notes evenly across all 3 octaves (uses high/low more)
    Sharps = 8,        // 36-key mode: shifts notes to use more Shift/Ctrl modifiers
}

impl From<u8> for NoteMode {
    fn from(value: u8) -> Self {
        match value {
            0 => NoteMode::Closest,
            1 => NoteMode::Quantize,
            2 => NoteMode::TransposeOnly,
            3 => NoteMode::Pentatonic,
            4 => NoteMode::Chromatic,
            5 => NoteMode::Raw,
            6 => NoteMode::Python,
            7 => NoteMode::Wide,
            8 => NoteMode::Sharps,
            _ => NoteMode::Closest,
        }
    }
}

/// Key mode - how many keys the instrument has
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum KeyMode {
    Keys21 = 0, // Standard 21 keys (7 notes × 3 octaves)
    Keys36 = 1, // 36 keys with Shift/Ctrl for sharps/flats
}

impl From<u8> for KeyMode {
    fn from(value: u8) -> Self {
        match value {
            0 => KeyMode::Keys21,
            1 => KeyMode::Keys36,
            _ => KeyMode::Keys21,
        }
    }
}

/// Band mode filter - how to filter notes for multiplayer
#[derive(Debug, Clone)]
pub enum BandFilter {
    /// Split mode: player plays every Nth note starting from slot
    Split { slot: usize, total_players: usize },
    /// Track mode: player plays only notes from a specific track
    Track { track_id: usize },
}

#[derive(Debug, Clone)]
pub struct MidiData {
    pub events: Vec<TimedEvent>,
    pub duration: f64,
    pub transpose: i32,
}

#[derive(Debug, Clone)]
pub struct TimedEvent {
    pub time_ms: u64,
    pub event_type: EventType,
    pub note: u8,
    pub track_id: usize, // Track index for band mode filtering
}

#[derive(Debug, Clone)]
pub enum EventType {
    NoteOn,
    NoteOff,
}

// 21-key mode: Basic keys for 3 octaves (7 notes each)
const LOW_KEYS: [&str; 7] = ["z", "x", "c", "v", "b", "n", "m"];
const MID_KEYS: [&str; 7] = ["a", "s", "d", "f", "g", "h", "j"];
const HIGH_KEYS: [&str; 7] = ["q", "w", "e", "r", "t", "y", "u"];

const ROOT_NOTE: i32 = 60; // C4

/// MIDI metadata for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiMetadata {
    pub duration: f64,     // seconds
    pub bpm: u16,          // beats per minute (initial tempo)
    pub note_count: u32,   // total note-on events
    pub note_density: f32, // notes per second
}

/// MIDI track information for band mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiTrackInfo {
    pub id: usize,           // track index
    pub name: String,        // track name (from MIDI metadata or generated)
    pub note_count: u32,     // number of notes in this track
    pub channel: Option<u8>, // MIDI channel (0-15) if consistent
}

/// Get all MIDI metadata in a single parse (efficient for bulk loading)
pub fn get_midi_metadata(path: &str) -> Result<MidiMetadata, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let smf = Smf::parse(&data).map_err(|e| e.to_string())?;

    let ticks_per_quarter = match smf.header.timing {
        midly::Timing::Metrical(tpq) => tpq.as_int() as f64,
        _ => 480.0,
    };

    let mut tempo_changes: Vec<(u64, f64)> = Vec::new();
    let mut max_ticks: u64 = 0;
    let mut note_count: u32 = 0;
    let mut initial_tempo: f64 = 500_000.0; // Default 120 BPM
    let mut found_initial_tempo = false;

    // Single pass: collect tempo, duration, and note count
    for track in &smf.tracks {
        let mut track_time_ticks: u64 = 0;
        for event in track {
            track_time_ticks += event.delta.as_int() as u64;

            match event.kind {
                TrackEventKind::Meta(midly::MetaMessage::Tempo(t)) => {
                    let tempo_val = t.as_int() as f64;
                    if !found_initial_tempo {
                        initial_tempo = tempo_val;
                        found_initial_tempo = true;
                    }
                    tempo_changes.push((track_time_ticks, tempo_val));
                }
                TrackEventKind::Midi {
                    message: MidiMessage::NoteOn { vel, .. },
                    ..
                } => {
                    if vel.as_int() > 0 {
                        note_count += 1;
                    }
                }
                _ => {}
            }
        }
        if track_time_ticks > max_ticks {
            max_ticks = track_time_ticks;
        }
    }
    tempo_changes.sort_by_key(|(time, _)| *time);

    // Calculate duration in seconds
    let mut result_ms = 0.0;
    let mut last_tick = 0u64;
    let mut current_tempo = 500_000.0;

    for &(change_tick, new_tempo) in &tempo_changes {
        if change_tick >= max_ticks {
            break;
        }
        let delta_ticks = change_tick - last_tick;
        result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;
        last_tick = change_tick;
        current_tempo = new_tempo;
    }

    let delta_ticks = max_ticks - last_tick;
    result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;

    let duration = result_ms / 1000.0; // seconds
    let bpm = (60_000_000.0 / initial_tempo).round() as u16;
    let note_density = if duration > 0.0 {
        note_count as f32 / duration as f32
    } else {
        0.0
    };

    Ok(MidiMetadata {
        duration,
        bpm,
        note_count,
        note_density,
    })
}

/// Clean track name - keep only printable ASCII chars (A-Z, a-z, 0-9, space, common punctuation)
fn clean_track_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| {
            c.is_ascii_alphanumeric()
                || *c == ' '
                || *c == '-'
                || *c == '_'
                || *c == '.'
                || *c == '('
                || *c == ')'
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Get track information from a MIDI file (for band mode)
pub fn get_midi_tracks(path: &str) -> Result<Vec<MidiTrackInfo>, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let smf = Smf::parse(&data).map_err(|e| e.to_string())?;

    let mut tracks = Vec::new();

    for (idx, track) in smf.tracks.iter().enumerate() {
        let mut name = String::new();
        let mut note_count: u32 = 0;
        let mut channels: std::collections::HashSet<u8> = std::collections::HashSet::new();

        for event in track {
            match event.kind {
                TrackEventKind::Meta(midly::MetaMessage::TrackName(n)) => {
                    name = clean_track_name(&String::from_utf8_lossy(n));
                }
                TrackEventKind::Meta(midly::MetaMessage::InstrumentName(n)) => {
                    if name.is_empty() {
                        name = clean_track_name(&String::from_utf8_lossy(n));
                    }
                }
                TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOn { vel, .. },
                } => {
                    if vel.as_int() > 0 {
                        note_count += 1;
                        channels.insert(channel.as_int());
                    }
                }
                _ => {}
            }
        }

        // Only include tracks with notes
        if note_count > 0 {
            let channel = if channels.len() == 1 {
                Some(*channels.iter().next().unwrap())
            } else {
                None
            };

            // Generate name if not found
            if name.is_empty() {
                name = format!("Track {}", idx + 1);
            }

            tracks.push(MidiTrackInfo {
                id: idx,
                name,
                note_count,
                channel,
            });
        }
    }

    Ok(tracks)
}

pub fn load_midi(path: &str) -> Result<MidiData, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let smf = Smf::parse(&data).map_err(|e| e.to_string())?;

    let mut events = Vec::new();
    let ticks_per_quarter = match smf.header.timing {
        midly::Timing::Metrical(tpq) => tpq.as_int() as f64,
        _ => 480.0, // Default
    };

    let mut tempo_changes: Vec<(u64, f64)> = Vec::new();

    // First pass: collect all tempo changes from all tracks
    for track in &smf.tracks {
        let mut track_time_ticks: u64 = 0;
        for event in track {
            track_time_ticks += event.delta.as_int() as u64;
            if let TrackEventKind::Meta(midly::MetaMessage::Tempo(t)) = event.kind {
                tempo_changes.push((track_time_ticks, t.as_int() as f64));
            }
        }
    }
    tempo_changes.sort_by_key(|(time, _)| *time);

    // Function to convert ticks to milliseconds with tempo changes
    let ticks_to_ms = |ticks: u64| -> u64 {
        let mut result_ms = 0.0;
        let mut last_tick = 0u64;
        let mut current_tempo = 500_000.0;

        for &(change_tick, new_tempo) in &tempo_changes {
            if change_tick >= ticks {
                break;
            }
            // Add time up to this tempo change
            let delta_ticks = change_tick - last_tick;
            result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;
            last_tick = change_tick;
            current_tempo = new_tempo;
        }

        // Add remaining time
        let delta_ticks = ticks - last_tick;
        result_ms += delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0;
        result_ms as u64
    };

    // Second pass: process all tracks with proper timing
    for (track_idx, track) in smf.tracks.iter().enumerate() {
        let mut track_time_ticks: u64 = 0;

        for event in track {
            track_time_ticks += event.delta.as_int() as u64;
            let time_ms = ticks_to_ms(track_time_ticks);

            if let TrackEventKind::Midi { message, .. } = event.kind {
                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        if vel > 0 {
                            events.push(TimedEvent {
                                time_ms,
                                event_type: EventType::NoteOn,
                                note: key.as_int(),
                                track_id: track_idx,
                            });
                        } else {
                            // Note on with velocity 0 is treated as note off
                            events.push(TimedEvent {
                                time_ms,
                                event_type: EventType::NoteOff,
                                note: key.as_int(),
                                track_id: track_idx,
                            });
                        }
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        events.push(TimedEvent {
                            time_ms,
                            event_type: EventType::NoteOff,
                            note: key.as_int(),
                            track_id: track_idx,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Sort events by time
    events.sort_by_key(|e| e.time_ms);

    // Calculate duration
    let duration = if !events.is_empty() {
        events.last().unwrap().time_ms as f64 / 1000.0
    } else {
        0.0
    };

    // Detect best transpose (port of Python heuristic)
    let transpose = detect_best_transpose(&events);
    println!("Detected transpose: {} semitones", transpose);

    Ok(MidiData {
        events,
        duration,
        transpose,
    })
}

fn detect_best_transpose(events: &[TimedEvent]) -> i32 {
    let instrument_notes = get_instrument_notes();

    let mut best_transpose = 0;
    let mut best_score = i32::MAX;

    // Test transpose values from -12 to +12
    for transpose in -12..=12 {
        let mut score = 0;

        for event in events {
            if matches!(event.event_type, EventType::NoteOn) {
                let transposed_note = (event.note as i32 + transpose) as i32;
                let normalized = normalize_into_range(transposed_note);

                // Find distance to nearest instrument note
                let mut min_distance = i32::MAX;
                for &inst_note in instrument_notes.iter() {
                    let distance = (inst_note - normalized).abs();
                    if distance < min_distance {
                        min_distance = distance;
                    }
                }
                score += min_distance;
            }
        }

        if score < best_score {
            best_score = score;
            best_transpose = transpose;
        }
    }

    best_transpose
}

#[inline]
fn get_instrument_notes() -> &'static [i32; 21] {
    &INSTRUMENT_NOTES
}

fn normalize_into_range(note: i32) -> i32 {
    // Match Python version exactly - simple octave shifting
    // Our instrument range: C3 (48) to B5 (83)
    let lo = INSTRUMENT_NOTES[0]; // 48
    let hi = INSTRUMENT_NOTES[20]; // 83

    let mut result = note;
    while result < lo {
        result += 12;
    }
    while result > hi {
        result -= 12;
    }
    result
}

// Pre-computed instrument notes for faster lookup
const INSTRUMENT_NOTES: [i32; 21] = [
    // Low octave (C3-B3): 48, 50, 52, 53, 55, 57, 59
    48, 50, 52, 53, 55, 57, 59, // Mid octave (C4-B4): 60, 62, 64, 65, 67, 69, 71
    60, 62, 64, 65, 67, 69, 71, // High octave (C5-B5): 72, 74, 76, 77, 79, 81, 83
    72, 74, 76, 77, 79, 81, 83,
];

fn note_to_key(note: i32, transpose: i32) -> String {
    // Match Python version exactly
    let target = normalize_into_range(note + transpose);

    let mut best_idx = 0;
    let mut best_dist = (INSTRUMENT_NOTES[0] - target).abs();

    for (i, &inst_note) in INSTRUMENT_NOTES.iter().enumerate() {
        let dist = (inst_note - target).abs();
        if dist < best_dist {
            best_idx = i;
            best_dist = dist;
        }
    }

    // Map index to key (21 keys total)
    const ALL_KEYS: [&str; 21] = [
        "z", "x", "c", "v", "b", "n", "m", // Low
        "a", "s", "d", "f", "g", "h", "j", // Mid
        "q", "w", "e", "r", "t", "y", "u", // High
    ];

    ALL_KEYS[best_idx].to_string()
}

/// Quantize mode - snap to exact scale notes only (no in-between approximation)
fn note_to_key_quantize(note: i32, transpose: i32) -> String {
    // Just use closest mode - same behavior
    note_to_key(note, transpose)
}

/// Transpose Only mode - direct semitone to key mapping within octave
fn note_to_key_transpose(note: i32, transpose: i32) -> String {
    let target = note + transpose;

    // Get semitone within octave (0-11)
    let semitone = ((target - ROOT_NOTE) % 12 + 12) % 12;

    // Determine octave
    let octave_offset = (target - ROOT_NOTE) / 12;
    let octave = (1 + octave_offset).clamp(0, 2) as usize;

    // Direct mapping: semitone 0-11 to key 0-6 (wrap around)
    // This gives a more "raw" feel
    let key_idx = (semitone * 7 / 12) as usize;

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

/// Pentatonic mode - map to pentatonic scale (5 notes per octave)
fn note_to_key_pentatonic(note: i32, transpose: i32) -> String {
    let normalized = normalize_into_range(note + transpose);

    // Get semitone and octave
    let semitone = ((normalized - ROOT_NOTE) % 12 + 12) % 12;
    let octave = if normalized < 60 {
        0
    } else if normalized < 72 {
        1
    } else {
        2
    };

    // Map to pentatonic: C(0), D(2), E(4), G(7), A(9) -> keys 0,1,2,4,5
    let key_idx = match semitone {
        0 | 1 => 0,     // C, C# -> do
        2 | 3 => 1,     // D, Eb -> re
        4 | 5 | 6 => 2, // E, F, F# -> mi
        7 | 8 => 4,     // G, G# -> so
        _ => 5,         // A, Bb, B -> la
    };

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

/// Chromatic mode - detailed mapping of all 12 semitones to closest natural key
fn note_to_key_chromatic(note: i32, transpose: i32) -> String {
    let normalized = normalize_into_range(note + transpose);

    // Get semitone and octave
    let semitone = ((normalized - ROOT_NOTE) % 12 + 12) % 12;
    let octave = if normalized < 60 {
        0
    } else if normalized < 72 {
        1
    } else {
        2
    };

    // Map 12 semitones to 7 keys
    let key_idx = match semitone {
        0 | 1 => 0, // C, C# -> do
        2 => 1,     // D -> re
        3 | 4 => 2, // Eb, E -> mi
        5 | 6 => 3, // F, F# -> fa
        7 | 8 => 4, // G, G# -> so
        9 => 5,     // A -> la
        _ => 6,     // Bb, B -> ti
    };

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

/// Raw mode - direct 1:1 mapping, no transpose, no processing
/// MIDI note modulo 21 maps directly to one of 21 keys
fn note_to_key_raw(note: i32) -> String {
    // Direct mapping: note % 21 gives key index 0-20
    let key_idx = ((note % 21) + 21) % 21; // Handle negative notes
    let all_keys = [
        LOW_KEYS.as_slice(),
        MID_KEYS.as_slice(),
        HIGH_KEYS.as_slice(),
    ]
    .concat();
    all_keys[key_idx as usize].to_string()
}

/// Wide mode - spread notes evenly across all 21 keys
/// Uses high and low rows more often by mapping the song's note range proportionally
fn note_to_key_wide(note: i32, transpose: i32) -> String {
    let target = note + transpose;

    // Map MIDI note range (roughly 36-96, typical piano range) to 21 keys
    // Piano middle C is MIDI 60, we want that around the middle keys
    // Low: 36-47 (C2-B2), Mid: 48-71 (C3-B4), High: 72-96 (C5-C7)

    // Use semitone position to determine octave more aggressively
    // Instead of normalizing into a narrow range, we keep the original octave feel
    let semitone = ((target % 12) + 12) % 12;

    // Determine octave based on actual note height
    // This uses the full range: notes below 54 go low, 54-66 go mid, above 66 go high
    let octave = if target < 54 {
        0 // Low row - anything below F#3
    } else if target < 66 {
        1 // Mid row - F#3 to F#4
    } else {
        2 // High row - anything above F#4
    };

    // Map semitone to key index (0-6)
    // Use a direct semitone-to-degree mapping
    let key_idx = match semitone {
        0 => 0,     // C -> do
        1 | 2 => 1, // C#, D -> re
        3 | 4 => 2, // D#, E -> mi
        5 => 3,     // F -> fa
        6 | 7 => 4, // F#, G -> so
        8 | 9 => 5, // G#, A -> la
        _ => 6,     // A#, B -> ti
    };

    match octave {
        0 => LOW_KEYS[key_idx].to_string(),
        1 => MID_KEYS[key_idx].to_string(),
        _ => HIGH_KEYS[key_idx].to_string(),
    }
}

/// Python mode - EXACT 1:1 copy of main.py logic
/// This is the proven working implementation
fn note_to_key_python(note: i32, transpose: i32) -> String {
    // EXACT copy from Python main.py
    // SCALE_INTERVALS = [0, 2, 4, 5, 7, 9, 11]
    // ROOT_NOTE = 60
    // LOW_SCALE = [48, 50, 52, 53, 55, 57, 59]
    // MID_SCALE = [60, 62, 64, 65, 67, 69, 71]
    // HIGH_SCALE = [72, 74, 76, 77, 79, 81, 83]
    // INSTRUMENT_NOTES = LOW_SCALE + MID_SCALE + HIGH_SCALE (21 notes)

    const PY_INSTRUMENT_NOTES: [i32; 21] = [
        48, 50, 52, 53, 55, 57, 59, // LOW_SCALE
        60, 62, 64, 65, 67, 69, 71, // MID_SCALE
        72, 74, 76, 77, 79, 81, 83, // HIGH_SCALE
    ];

    const PY_KEYS: [&str; 21] = [
        "z", "x", "c", "v", "b", "n", "m", // LOW_KEYS
        "a", "s", "d", "f", "g", "h", "j", // MID_KEYS
        "q", "w", "e", "r", "t", "y", "u", // HIGH_KEYS
    ];

    // normalize_into_range - EXACT Python logic
    let lo = PY_INSTRUMENT_NOTES[0]; // 48
    let hi = PY_INSTRUMENT_NOTES[20]; // 83

    let mut target = note + transpose;
    while target < lo {
        target += 12;
    }
    while target > hi {
        target -= 12;
    }

    // note_to_key - EXACT Python logic
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

/// Convert a semitone (0-11) and octave (0-2) to a 36-key string
/// Natural notes use normal keys, accidentals use Shift/Ctrl modifiers
fn semitone_to_key_36(semitone: i32, octave: usize) -> String {
    match semitone {
        // Natural notes - normal keys
        0 => match octave {
            0 => "z",
            1 => "a",
            _ => "q",
        }
        .to_string(), // C (do)
        2 => match octave {
            0 => "x",
            1 => "s",
            _ => "w",
        }
        .to_string(), // D (re)
        4 => match octave {
            0 => "c",
            1 => "d",
            _ => "e",
        }
        .to_string(), // E (mi)
        5 => match octave {
            0 => "v",
            1 => "f",
            _ => "r",
        }
        .to_string(), // F (fa)
        7 => match octave {
            0 => "b",
            1 => "g",
            _ => "t",
        }
        .to_string(), // G (so)
        9 => match octave {
            0 => "n",
            1 => "h",
            _ => "y",
        }
        .to_string(), // A (la)
        11 => match octave {
            0 => "m",
            1 => "j",
            _ => "u",
        }
        .to_string(), // B (ti)

        // Accidentals - Shift or Ctrl + key
        1 => match octave {
            0 => "shift+z",
            1 => "shift+a",
            _ => "shift+q",
        }
        .to_string(), // C# (#1)
        3 => match octave {
            0 => "ctrl+c",
            1 => "ctrl+d",
            _ => "ctrl+e",
        }
        .to_string(), // D#/Eb (b3)
        6 => match octave {
            0 => "shift+v",
            1 => "shift+f",
            _ => "shift+r",
        }
        .to_string(), // F# (#4)
        8 => match octave {
            0 => "shift+b",
            1 => "shift+g",
            _ => "shift+t",
        }
        .to_string(), // G# (#5)
        10 => match octave {
            0 => "ctrl+m",
            1 => "ctrl+j",
            _ => "ctrl+u",
        }
        .to_string(), // A#/Bb (b7)

        _ => "a".to_string(), // Fallback
    }
}

/// Calculate octave (0=low, 1=mid, 2=high) for 36-key mode
fn get_octave_36(target: i32) -> usize {
    // C3=48, C4=60, C5=72
    // Low octave: <60, Mid: 60-71, High: >=72
    if target < 60 {
        0
    } else if target < 72 {
        1
    } else {
        2
    }
}

/// 36-key Closest mode - find closest available chromatic note
fn note_to_key_36_closest(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);
    semitone_to_key_36(semitone, octave)
}

/// 36-key Quantize mode - snap to exact major scale notes only
fn note_to_key_36_quantize(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);

    // Snap to nearest major scale note (0, 2, 4, 5, 7, 9, 11)
    let quantized = match semitone {
        0 | 1 => 0,  // C, C# -> C
        2 | 3 => 2,  // D, D# -> D
        4 => 4,      // E -> E
        5 | 6 => 5,  // F, F# -> F
        7 | 8 => 7,  // G, G# -> G
        9 | 10 => 9, // A, A# -> A
        11 => 11,    // B -> B
        _ => 0,
    };

    semitone_to_key_36(quantized, octave)
}

/// 36-key TransposeOnly mode - direct semitone mapping
fn note_to_key_36_transpose(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);
    semitone_to_key_36(semitone, octave)
}

/// 36-key Pentatonic mode - map to pentatonic scale
/// Pentatonic: C, D, E, G, A (semitones 0, 2, 4, 7, 9)
fn note_to_key_36_pentatonic(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);

    // Map to nearest pentatonic note
    let penta = match semitone {
        0 | 1 => 0,       // C, C# -> C
        2 | 3 => 2,       // D, D# -> D
        4 | 5 | 6 => 4,   // E, F, F# -> E
        7 | 8 => 7,       // G, G# -> G
        9 | 10 | 11 => 9, // A, A#, B -> A
        _ => 0,
    };

    semitone_to_key_36(penta, octave)
}

/// 36-key Chromatic mode - full chromatic mapping (same as closest for 36-key)
fn note_to_key_36_chromatic(note: i32, transpose: i32) -> String {
    note_to_key_36_closest(note, transpose)
}

/// 36-key Raw mode - direct 1:1 mapping, no transpose
fn note_to_key_36_raw(note: i32) -> String {
    // 36 total keys: 12 per octave × 3 octaves
    let key_idx = ((note % 36) + 36) % 36;
    let octave = (key_idx / 12) as usize;
    let semitone = key_idx % 12;

    semitone_to_key_36(semitone, octave)
}

/// 36-key Wide mode - spread notes using wider octave boundaries
fn note_to_key_36_wide(note: i32, transpose: i32) -> String {
    let target = note + transpose;
    let semitone = ((target % 12) + 12) % 12;

    // Use wider octave boundaries (same as 21-key Wide)
    let octave = if target < 54 {
        0 // Low row
    } else if target < 66 {
        1 // Mid row
    } else {
        2 // High row
    };

    semitone_to_key_36(semitone, octave)
}

/// 36-key Sharps mode - shifts notes to use more Shift/Ctrl modifiers
/// Adds +1 semitone so natural notes become sharps (C→C#, D→D#, etc.)
fn note_to_key_36_sharps(note: i32, transpose: i32) -> String {
    // Shift by +1 semitone to convert naturals to sharps
    let target = note + transpose + 1;
    let semitone = ((target % 12) + 12) % 12;
    let octave = get_octave_36(target);
    semitone_to_key_36(semitone, octave)
}

pub fn play_midi(
    midi_data: MidiData,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    loop_mode: Arc<AtomicBool>,
    note_mode: Arc<AtomicU8>,
    key_mode: Arc<AtomicU8>,
    octave_shift: Arc<std::sync::atomic::AtomicI8>,
    speed: Arc<std::sync::atomic::AtomicU16>,
    current_position: Arc<std::sync::Mutex<f64>>,
    seek_offset: Arc<std::sync::Mutex<f64>>,
    band_filter: Arc<std::sync::Mutex<Option<BandFilter>>>,
    window: Window,
) {
    // Log band mode if active at start
    if let Some(ref filter) = *band_filter.lock().unwrap() {
        match filter {
            BandFilter::Split {
                slot,
                total_players,
            } => {
                println!(
                    "[BAND] Split mode: playing note {} of every {} notes",
                    slot + 1,
                    total_players
                );
            }
            BandFilter::Track { track_id } => {
                println!("[BAND] Track mode: playing track {}", track_id);
            }
        }
    }

    // Spawn a separate thread for progress updates
    let is_playing_progress = Arc::clone(&is_playing);
    let is_paused_progress = Arc::clone(&is_paused);
    let current_position_progress = Arc::clone(&current_position);
    let window_progress = window.clone();

    std::thread::spawn(move || {
        while is_playing_progress.load(Ordering::SeqCst) {
            if !is_paused_progress.load(Ordering::SeqCst) {
                let position = *current_position_progress.lock().unwrap();
                let _ = window_progress.emit("playback-progress", position);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    loop {
        // Get current seek offset (reset to 0 on loop)
        let offset_ms = (*seek_offset.lock().unwrap() * 1000.0) as u64;

        // Track which key is pressed for each MIDI note (note -> key that was pressed)
        let _note_to_pressed_key: std::collections::HashMap<u8, String> =
            std::collections::HashMap::new();
        // Track reference count for each key (multiple notes might map to same key)
        let key_active_count: std::collections::HashMap<String, i32> =
            std::collections::HashMap::new();

        // Helper to release all keys and reset modifier counts
        let release_all_keys = |key_active_count: &std::collections::HashMap<String, i32>| {
            for (key, count) in key_active_count {
                if *count > 0 {
                    crate::keyboard::key_up(key);
                }
            }
            // Reset modifier reference counts when stopping
            crate::keyboard::reset_modifier_counts();
        };

        // Track song position in milliseconds (not affected by speed changes)
        let mut song_position_ms: u64 = offset_ms;
        let mut last_event_time = Instant::now();

        // Counter for split mode note filtering
        let mut note_on_counter: usize = 0;

        for event in &midi_data.events {
            if event.time_ms < offset_ms {
                continue;
            }

            if !is_playing.load(Ordering::SeqCst) {
                release_all_keys(&key_active_count);
                return;
            }

            // Calculate delta from last processed position to this event (in song time)
            let delta_song_ms = event.time_ms.saturating_sub(song_position_ms);

            // Wait for the delta time, adjusted by current speed
            if delta_song_ms > 0 {
                let mut remaining_song_ms = delta_song_ms as f64;

                while remaining_song_ms > 0.0 {
                    if !is_playing.load(Ordering::SeqCst) {
                        release_all_keys(&key_active_count);
                        return;
                    }

                    // Handle pause
                    if is_paused.load(Ordering::SeqCst) {
                        while is_paused.load(Ordering::SeqCst) && is_playing.load(Ordering::SeqCst)
                        {
                            std::thread::sleep(Duration::from_millis(50));
                            if !is_playing.load(Ordering::SeqCst) {
                                release_all_keys(&key_active_count);
                                return;
                            }
                        }
                        last_event_time = Instant::now();
                        continue;
                    }

                    // Get current speed (stored as speed * 100)
                    let current_speed = speed.load(Ordering::SeqCst) as f64 / 100.0;

                    // Calculate real time to wait based on speed
                    // sleep for a small chunk and update
                    let sleep_ms = 2.0_f64.min(remaining_song_ms / current_speed);
                    std::thread::sleep(Duration::from_micros((sleep_ms * 1000.0) as u64));

                    let elapsed = last_event_time.elapsed();
                    last_event_time = Instant::now();

                    // Convert real elapsed time to song time
                    let song_ms_passed = elapsed.as_secs_f64() * 1000.0 * current_speed;
                    remaining_song_ms -= song_ms_passed;

                    // Update current position
                    let new_pos = (event.time_ms as f64 - remaining_song_ms.max(0.0)) / 1000.0;
                    *current_position.lock().unwrap() = new_pos;
                }
            }

            // Update song position to this event's time
            song_position_ms = event.time_ms;
            last_event_time = Instant::now();

            // Get key based on key mode and note calculation mode (read in realtime for live switching)
            let current_key_mode = KeyMode::from(key_mode.load(Ordering::SeqCst));
            let current_note_mode = NoteMode::from(note_mode.load(Ordering::SeqCst));
            // Get octave shift in semitones (1 octave = 12 semitones)
            let shift_semitones = octave_shift.load(Ordering::SeqCst) as i32 * 12;
            let total_transpose = midi_data.transpose + shift_semitones;

            // Select key mapping based on key mode and note mode
            let key = match current_key_mode {
                KeyMode::Keys36 => {
                    // 36-key mode - use note mode with modifier keys
                    match current_note_mode {
                        NoteMode::Closest => {
                            note_to_key_36_closest(event.note as i32, total_transpose)
                        }
                        NoteMode::Quantize => {
                            note_to_key_36_quantize(event.note as i32, total_transpose)
                        }
                        NoteMode::TransposeOnly => {
                            note_to_key_36_transpose(event.note as i32, total_transpose)
                        }
                        NoteMode::Pentatonic => {
                            note_to_key_36_pentatonic(event.note as i32, total_transpose)
                        }
                        NoteMode::Chromatic => {
                            note_to_key_36_chromatic(event.note as i32, total_transpose)
                        }
                        NoteMode::Raw => note_to_key_36_raw(event.note as i32 + shift_semitones),
                        NoteMode::Python => note_to_key_python(event.note as i32, total_transpose),
                        NoteMode::Wide => note_to_key_36_wide(event.note as i32, total_transpose),
                        NoteMode::Sharps => {
                            note_to_key_36_sharps(event.note as i32, total_transpose)
                        }
                    }
                }
                KeyMode::Keys21 => {
                    // 21-key mode - use note mode to determine mapping
                    match current_note_mode {
                        NoteMode::Closest => note_to_key(event.note as i32, total_transpose),
                        NoteMode::Quantize => {
                            note_to_key_quantize(event.note as i32, total_transpose)
                        }
                        NoteMode::TransposeOnly => {
                            note_to_key_transpose(event.note as i32, total_transpose)
                        }
                        NoteMode::Pentatonic => {
                            note_to_key_pentatonic(event.note as i32, total_transpose)
                        }
                        NoteMode::Chromatic => {
                            note_to_key_chromatic(event.note as i32, total_transpose)
                        }
                        NoteMode::Raw => note_to_key_raw(event.note as i32 + shift_semitones),
                        NoteMode::Python => note_to_key_python(event.note as i32, total_transpose),
                        NoteMode::Wide => note_to_key_wide(event.note as i32, total_transpose),
                        NoteMode::Sharps => note_to_key(event.note as i32, total_transpose), // Falls back to Closest in 21-key
                    }
                }
            };

            match event.event_type {
                EventType::NoteOn => {
                    // Check band filter - read live for instant track switching
                    let current_filter = band_filter.lock().unwrap().clone();
                    let should_play = match &current_filter {
                        Some(BandFilter::Split {
                            slot,
                            total_players,
                        }) => {
                            // In split mode, play every Nth note starting from slot
                            let play = (note_on_counter % total_players) == *slot;
                            note_on_counter += 1;
                            play
                        }
                        Some(BandFilter::Track { track_id }) => {
                            // Track mode: only play notes from the assigned track
                            event.track_id == *track_id
                        }
                        None => true, // No filter, play all
                    };

                    if should_play {
                        // Simple press-release for each note (game doesn't need hold)
                        crate::keyboard::key_down(&key);
                        crate::keyboard::key_up(&key);

                        // Emit note event for visualizer
                        let _ = window.emit("note-event", &key);
                    }
                }
                EventType::NoteOff => {
                    // Ignore note off - we already released on note on
                }
            }
        }

        // Release all remaining keys
        release_all_keys(&key_active_count);

        if !loop_mode.load(Ordering::SeqCst) {
            break;
        }

        // Reset position to 0 for loop restart
        *seek_offset.lock().unwrap() = 0.0;
        *current_position.lock().unwrap() = 0.0;

        std::thread::sleep(Duration::from_millis(500));
    }

    is_playing.store(false, Ordering::SeqCst);
    let _ = window.emit("playback-ended", ());
}
