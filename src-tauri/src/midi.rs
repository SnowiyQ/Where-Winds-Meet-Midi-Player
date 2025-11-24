use midly::{Smf, TrackEventKind, MidiMessage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Window, Emitter};

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
}

#[derive(Debug, Clone)]
pub enum EventType {
    NoteOn,
    NoteOff,
}

// Key layout configuration - matching Python implementation
const LOW_KEYS: [&str; 7] = ["z", "x", "c", "v", "b", "n", "m"];
const MID_KEYS: [&str; 7] = ["a", "s", "d", "f", "g", "h", "j"];
const HIGH_KEYS: [&str; 7] = ["q", "w", "e", "r", "t", "y", "u"];

const SCALE_INTERVALS: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const ROOT_NOTE: i32 = 60; // C4

pub fn load_midi(path: &str) -> Result<MidiData, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let smf = Smf::parse(&data).map_err(|e| e.to_string())?;

    let mut events = Vec::new();
    let mut current_time_ms: u64 = 0;
    let ticks_per_quarter = match smf.header.timing {
        midly::Timing::Metrical(tpq) => tpq.as_int() as f64,
        _ => 480.0, // Default
    };

    let mut tempo = 500_000.0; // Default tempo (120 BPM)
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
            result_ms += (delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0);
            last_tick = change_tick;
            current_tempo = new_tempo;
        }

        // Add remaining time
        let delta_ticks = ticks - last_tick;
        result_ms += (delta_ticks as f64 / ticks_per_quarter * current_tempo / 1000.0);
        result_ms as u64
    };

    // Second pass: process all tracks with proper timing
    for track in &smf.tracks {
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
                            });
                        } else {
                            // Note on with velocity 0 is treated as note off
                            events.push(TimedEvent {
                                time_ms,
                                event_type: EventType::NoteOff,
                                note: key.as_int(),
                            });
                        }
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        events.push(TimedEvent {
                            time_ms,
                            event_type: EventType::NoteOff,
                            note: key.as_int(),
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
                for inst_note in &instrument_notes {
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

fn get_instrument_notes() -> Vec<i32> {
    let mut notes = Vec::new();

    // Low octave
    for interval in SCALE_INTERVALS {
        notes.push(ROOT_NOTE - 12 + interval);
    }

    // Mid octave
    for interval in SCALE_INTERVALS {
        notes.push(ROOT_NOTE + interval);
    }

    // High octave
    for interval in SCALE_INTERVALS {
        notes.push(ROOT_NOTE + 12 + interval);
    }

    notes
}

fn normalize_into_range(note: i32) -> i32 {
    let instrument_notes = get_instrument_notes();
    let lo = instrument_notes[0];
    let hi = instrument_notes[instrument_notes.len() - 1];

    let mut normalized = note;
    while normalized < lo {
        normalized += 12;
    }
    while normalized > hi {
        normalized -= 12;
    }

    normalized
}

fn note_to_key(note: i32, transpose: i32) -> String {
    let target = normalize_into_range(note + transpose);
    let instrument_notes = get_instrument_notes();

    let mut best_idx = 0;
    let mut best_dist = (instrument_notes[0] - target).abs();

    for (i, &inst_note) in instrument_notes.iter().enumerate() {
        let dist = (inst_note - target).abs();
        if dist < best_dist {
            best_idx = i;
            best_dist = dist;
        }
    }

    // Map index to key
    let all_keys = [LOW_KEYS.as_slice(), MID_KEYS.as_slice(), HIGH_KEYS.as_slice()].concat();
    let key = all_keys[best_idx].to_string();

    // Debug first few mappings
    static mut DEBUG_COUNT: i32 = 0;
    unsafe {
        if DEBUG_COUNT < 5 {
            println!("Note {} + transpose {} = target {}, matched {}, key: {}",
                note, transpose, target, instrument_notes[best_idx], key);
            DEBUG_COUNT += 1;
        }
    }

    key
}

pub fn play_midi(
    midi_data: MidiData,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    loop_mode: Arc<AtomicBool>,
    current_position: Arc<std::sync::Mutex<f64>>,
    seek_offset: Arc<std::sync::Mutex<f64>>,
    window: Window,
) {
    // Get the seek offset
    let offset_ms = (*seek_offset.lock().unwrap() * 1000.0) as u64;

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
            std::thread::sleep(Duration::from_millis(100)); // Update 10 times per second
        }
    });

    loop {
        let start_time = Instant::now();
        let mut key_active_count: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

        for event in &midi_data.events {
            // Skip events before the seek offset
            if event.time_ms < offset_ms {
                continue;
            }

            // Check if we should stop
            if !is_playing.load(Ordering::SeqCst) {
                // Release all pressed keys
                for (key, count) in &key_active_count {
                    if *count > 0 {
                        crate::keyboard::key_up(key);
                    }
                }
                return;
            }

            // Handle pause
            while is_paused.load(Ordering::SeqCst) && is_playing.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(50));
                // Check if stopped during pause
                if !is_playing.load(Ordering::SeqCst) {
                    for (key, count) in &key_active_count {
                        if *count > 0 {
                            crate::keyboard::key_up(key);
                        }
                    }
                    return;
                }
            }

            // Wait until event time, but check for stop more frequently
            // Adjust target time to account for seek offset
            let target_time = Duration::from_millis(event.time_ms - offset_ms);
            while start_time.elapsed() < target_time {
                if !is_playing.load(Ordering::SeqCst) {
                    // Release all pressed keys immediately
                    for (key, count) in &key_active_count {
                        if *count > 0 {
                            crate::keyboard::key_up(key);
                        }
                    }
                    return;
                }
                std::thread::sleep(Duration::from_millis(1));
            }

            // Update current position (accounting for offset)
            *current_position.lock().unwrap() = start_time.elapsed().as_secs_f64() + (offset_ms as f64 / 1000.0);

            // Progress updates are now handled by the separate progress thread

            // Process the event
            let key = note_to_key(event.note as i32, midi_data.transpose);

            match event.event_type {
                EventType::NoteOn => {
                    let count = key_active_count.entry(key.clone()).or_insert(0);
                    if *count == 0 {
                        crate::keyboard::key_down(&key);
                    }
                    *count += 1;
                }
                EventType::NoteOff => {
                    if let Some(count) = key_active_count.get_mut(&key) {
                        if *count > 0 {
                            *count -= 1;
                            if *count == 0 {
                                crate::keyboard::key_up(&key);
                            }
                        }
                    }
                }
            }
        }

        // Release all remaining keys
        for (key, count) in &key_active_count {
            if *count > 0 {
                crate::keyboard::key_up(key);
            }
        }

        // Check if we should loop
        if !loop_mode.load(Ordering::SeqCst) {
            break;
        }

        // Small delay before looping
        std::thread::sleep(Duration::from_millis(500));
    }

    is_playing.store(false, Ordering::SeqCst);
    let _ = window.emit("playback-ended", ());
}