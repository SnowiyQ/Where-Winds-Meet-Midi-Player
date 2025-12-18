use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU16, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Window;

use crate::midi::{BandFilter, EventType, KeyMode, NoteMode};
use crate::midi_input::MidiInputState;

/// Note event for visualizer (simplified for frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizerNote {
    pub time_ms: u64,     // Start time in ms
    pub duration_ms: u64, // Duration in ms
    pub note: u8,         // MIDI note number
    pub key_index: u8,    // Key index (0-20 for 21 keys)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub current_position: f64,
    pub total_duration: f64,
    pub current_file: Option<String>,
    pub loop_mode: bool,
    pub note_mode: NoteMode,
    pub key_mode: KeyMode,
    pub octave_shift: i8,
    pub speed: f64,
}

pub struct AppState {
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    loop_mode: Arc<AtomicBool>,
    note_mode: Arc<AtomicU8>,
    key_mode: Arc<AtomicU8>,
    octave_shift: Arc<AtomicI8>,
    speed: Arc<AtomicU16>, // Stored as speed * 100 (e.g., 100 = 1.0x, 50 = 0.5x)
    current_position: Arc<std::sync::Mutex<f64>>,
    total_duration: Arc<std::sync::Mutex<f64>>,
    current_file: Arc<std::sync::Mutex<Option<String>>>,
    playback_start: Arc<std::sync::Mutex<Option<Instant>>>,
    midi_data: Arc<std::sync::Mutex<Option<crate::midi::MidiData>>>,
    seek_offset: Arc<std::sync::Mutex<f64>>,
    // Band mode filter
    band_filter: Arc<std::sync::Mutex<Option<BandFilter>>>,
    // Live MIDI input state
    pub midi_input_state: Arc<std::sync::Mutex<MidiInputState>>,
    pub is_live_mode_active: Arc<AtomicBool>,
    pub live_transpose: Arc<AtomicI8>, // Separate transpose for live mode
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            is_playing: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            loop_mode: Arc::new(AtomicBool::new(false)),
            note_mode: Arc::new(AtomicU8::new(NoteMode::Python as u8)),
            key_mode: Arc::new(AtomicU8::new(KeyMode::Keys21 as u8)),
            octave_shift: Arc::new(AtomicI8::new(0)),
            speed: Arc::new(AtomicU16::new(100)), // Default 1.0x speed
            current_position: Arc::new(std::sync::Mutex::new(0.0)),
            total_duration: Arc::new(std::sync::Mutex::new(0.0)),
            current_file: Arc::new(std::sync::Mutex::new(None)),
            playback_start: Arc::new(std::sync::Mutex::new(None)),
            midi_data: Arc::new(std::sync::Mutex::new(None)),
            seek_offset: Arc::new(std::sync::Mutex::new(0.0)),
            band_filter: Arc::new(std::sync::Mutex::new(None)),
            // Live MIDI input
            midi_input_state: Arc::new(std::sync::Mutex::new(MidiInputState::new())),
            is_live_mode_active: Arc::new(AtomicBool::new(false)),
            live_transpose: Arc::new(AtomicI8::new(0)),
        }
    }

    // Live MIDI input getters
    pub fn get_midi_input_state(&self) -> Arc<std::sync::Mutex<MidiInputState>> {
        Arc::clone(&self.midi_input_state)
    }

    pub fn get_is_live_mode_active(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_live_mode_active)
    }

    pub fn get_note_mode_arc(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.note_mode)
    }

    pub fn get_key_mode_arc(&self) -> Arc<AtomicU8> {
        Arc::clone(&self.key_mode)
    }

    pub fn get_octave_shift_arc(&self) -> Arc<AtomicI8> {
        Arc::clone(&self.octave_shift)
    }

    pub fn get_live_transpose(&self) -> Arc<AtomicI8> {
        Arc::clone(&self.live_transpose)
    }

    pub fn set_live_transpose(&self, value: i8) {
        self.live_transpose
            .store(value.clamp(-12, 12), Ordering::SeqCst);
    }

    pub fn set_band_filter(
        &mut self,
        mode: String,
        slot: usize,
        total_players: usize,
        track_id: Option<usize>,
    ) {
        let filter = if mode == "split" {
            Some(BandFilter::Split {
                slot,
                total_players,
            })
        } else if mode == "track" {
            track_id.map(|id| BandFilter::Track { track_id: id })
        } else {
            None
        };
        *self.band_filter.lock().unwrap() = filter;
    }

    #[allow(dead_code)]
    pub fn clear_band_filter(&mut self) {
        *self.band_filter.lock().unwrap() = None;
    }

    pub fn load_midi(&mut self, path: &str) -> Result<(), String> {
        let midi_data = crate::midi::load_midi(path)?;

        *self.total_duration.lock().unwrap() = midi_data.duration;
        *self.current_file.lock().unwrap() = Some(path.to_string());
        *self.midi_data.lock().unwrap() = Some(midi_data);
        // Reset seek offset and position for new song
        *self.seek_offset.lock().unwrap() = 0.0;
        *self.current_position.lock().unwrap() = 0.0;

        Ok(())
    }

    pub fn start_playback(&mut self, window: Window) -> Result<(), String> {
        if let Some(midi_data) = self.midi_data.lock().unwrap().clone() {
            self.is_playing.store(true, Ordering::SeqCst);
            self.is_paused.store(false, Ordering::SeqCst);
            let offset = *self.seek_offset.lock().unwrap();
            *self.playback_start.lock().unwrap() = Some(Instant::now());
            *self.current_position.lock().unwrap() = offset;

            // Clone Arc references for the thread
            let is_playing = Arc::clone(&self.is_playing);
            let is_paused = Arc::clone(&self.is_paused);
            let loop_mode = Arc::clone(&self.loop_mode);
            let note_mode = Arc::clone(&self.note_mode);
            let key_mode = Arc::clone(&self.key_mode);
            let octave_shift = Arc::clone(&self.octave_shift);
            let speed = Arc::clone(&self.speed);
            let current_position = Arc::clone(&self.current_position);
            let seek_offset = Arc::clone(&self.seek_offset);
            // Pass Arc reference for live track switching
            let band_filter = Arc::clone(&self.band_filter);

            std::thread::spawn(move || {
                crate::midi::play_midi(
                    midi_data,
                    is_playing,
                    is_paused,
                    loop_mode,
                    note_mode,
                    key_mode,
                    octave_shift,
                    speed,
                    current_position,
                    seek_offset,
                    band_filter,
                    window,
                );
            });

            Ok(())
        } else {
            Err("No MIDI file loaded".to_string())
        }
    }

    /// Update band filter live during playback
    pub fn update_band_filter_live(&self, track_id: Option<usize>) {
        let filter = track_id.map(|id| BandFilter::Track { track_id: id });
        *self.band_filter.lock().unwrap() = filter;
    }

    pub fn set_note_mode(&mut self, mode: NoteMode) {
        self.note_mode.store(mode as u8, Ordering::SeqCst);
    }

    pub fn get_note_mode(&self) -> NoteMode {
        NoteMode::from(self.note_mode.load(Ordering::SeqCst))
    }

    pub fn set_key_mode(&mut self, mode: KeyMode) {
        self.key_mode.store(mode as u8, Ordering::SeqCst);
    }

    pub fn get_key_mode(&self) -> KeyMode {
        KeyMode::from(self.key_mode.load(Ordering::SeqCst))
    }

    pub fn set_octave_shift(&mut self, shift: i8) {
        // Clamp to -2 to +2 octaves
        let clamped = shift.clamp(-2, 2);
        self.octave_shift.store(clamped, Ordering::SeqCst);
    }

    pub fn get_octave_shift(&self) -> i8 {
        self.octave_shift.load(Ordering::SeqCst)
    }

    pub fn set_speed(&mut self, speed: f64) {
        // Clamp to 0.25x - 2.0x range, store as integer (speed * 100)
        let clamped = (speed.clamp(0.25, 2.0) * 100.0) as u16;
        self.speed.store(clamped, Ordering::SeqCst);
    }

    pub fn get_speed(&self) -> f64 {
        self.speed.load(Ordering::SeqCst) as f64 / 100.0
    }

    pub fn toggle_pause(&mut self) {
        if self.is_playing.load(Ordering::SeqCst) {
            let paused = !self.is_paused.load(Ordering::SeqCst);
            self.is_paused.store(paused, Ordering::SeqCst);
        }
    }

    pub fn stop_playback(&mut self) {
        self.is_playing.store(false, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        *self.current_position.lock().unwrap() = 0.0;
        *self.playback_start.lock().unwrap() = None;

        // Wait for the playback thread to detect the stop flag and clean up
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    pub fn set_loop_mode(&mut self, enabled: bool) {
        self.loop_mode.store(enabled, Ordering::SeqCst);
    }

    pub fn seek(&mut self, position: f64, window: Window) -> Result<(), String> {
        let was_paused = self.is_paused.load(Ordering::SeqCst);

        if self.is_playing.load(Ordering::SeqCst) {
            // Store the seek position
            *self.seek_offset.lock().unwrap() = position;

            // Restart playback from the new position
            self.stop_playback();
            self.start_playback(window)?;

            // Restore paused state if it was paused before seeking
            if was_paused {
                self.is_paused.store(true, Ordering::SeqCst);
            }
        } else {
            // Just set the position if not playing
            *self.current_position.lock().unwrap() = position;
            *self.seek_offset.lock().unwrap() = position;
        }
        Ok(())
    }

    pub fn get_playback_state(&self) -> PlaybackState {
        let mut position = *self.current_position.lock().unwrap();

        // Update position based on playback time if playing
        if self.is_playing.load(Ordering::SeqCst) && !self.is_paused.load(Ordering::SeqCst) {
            if let Some(start_time) = *self.playback_start.lock().unwrap() {
                position = start_time.elapsed().as_secs_f64();
            }
        }

        PlaybackState {
            is_playing: self.is_playing.load(Ordering::SeqCst),
            is_paused: self.is_paused.load(Ordering::SeqCst),
            current_position: position,
            total_duration: *self.total_duration.lock().unwrap(),
            current_file: self.current_file.lock().unwrap().clone(),
            loop_mode: self.loop_mode.load(Ordering::SeqCst),
            note_mode: self.get_note_mode(),
            key_mode: self.get_key_mode(),
            octave_shift: self.get_octave_shift(),
            speed: self.get_speed(),
        }
    }

    /// Get note events for visualizer - only shows actual key presses (21 keys)
    pub fn get_visualizer_notes(&self) -> Vec<VisualizerNote> {
        let midi_data = self.midi_data.lock().unwrap();
        if midi_data.is_none() {
            return Vec::new();
        }

        let midi = midi_data.as_ref().unwrap();
        let transpose = midi.transpose;
        let mut notes: Vec<VisualizerNote> = Vec::new();

        // Only collect note_on events (game does instant tap, not hold)
        // and map to the 21 game keys
        for event in &midi.events {
            if let EventType::NoteOn = event.event_type {
                let key_index = Self::note_to_key_index(event.note as i32, transpose);

                notes.push(VisualizerNote {
                    time_ms: event.time_ms,
                    duration_ms: 80, // Fixed short duration for tap visualization
                    note: event.note,
                    key_index,
                });
            }
        }

        // Sort by time
        notes.sort_by_key(|n| n.time_ms);

        // Remove duplicate key presses at same time (within 10ms)
        let mut filtered: Vec<VisualizerNote> = Vec::new();
        for note in notes {
            let dominated = filtered.iter().any(|existing| {
                existing.key_index == note.key_index
                    && (note.time_ms as i64 - existing.time_ms as i64).abs() < 10
            });
            if !dominated {
                filtered.push(note);
            }
        }

        filtered
    }

    /// Map MIDI note to key index (0-20)
    fn note_to_key_index(note: i32, transpose: i32) -> u8 {
        const INSTRUMENT_NOTES: [i32; 21] = [
            48, 50, 52, 53, 55, 57, 59, 60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83,
        ];

        // Normalize into range
        let lo = INSTRUMENT_NOTES[0];
        let hi = INSTRUMENT_NOTES[20];
        let mut target = note + transpose;
        while target < lo {
            target += 12;
        }
        while target > hi {
            target -= 12;
        }

        // Find closest key
        let mut best_idx: u8 = 0;
        let mut best_dist = (INSTRUMENT_NOTES[0] - target).abs();
        for (i, &inst_note) in INSTRUMENT_NOTES.iter().enumerate() {
            let dist = (inst_note - target).abs();
            if dist < best_dist {
                best_idx = i as u8;
                best_dist = dist;
            }
        }
        best_idx
    }
}
