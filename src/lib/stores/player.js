import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Player state
export const isPlaying = writable(false);
export const isPaused = writable(false);
export const currentPosition = writable(0);
export const totalDuration = writable(0);
export const currentFile = writable(null);
export const loopMode = writable(false);

// Playlist state
export const midiFiles = writable([]);
export const playlist = writable([]);
export const currentIndex = writable(0);

// UI state
export const isDraggable = writable(true);
export const isMinimized = writable(false);
export const smartPause = writable(true);

let smartPauseCooldownUntil = 0;

// Derived states
export const progress = derived(
  [currentPosition, totalDuration],
  ([$position, $duration]) => {
    if ($duration === 0) return 0;
    return ($position / $duration) * 100;
  }
);

export const formatTime = (seconds) => {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
};

// Load MIDI files from album folder
export async function loadMidiFiles() {
  try {
    const files = await invoke('load_midi_files');
    midiFiles.set(files);
    playlist.set([]); // Start with empty queue
  } catch (error) {
    console.error('Failed to load MIDI files:', error);
  }
}

// Play a MIDI file
export async function playMidi(path) {
  try {
    delaySmartPause();
    if (get(isPlaying)) {
      await stopPlayback();
    }
    await invoke('play_midi', { path });
    // Focus is now handled in the backend after playback starts
    await refreshPlaybackState();
    isPlaying.set(true);
    isPaused.set(false);
    currentFile.set(path);
  } catch (error) {
    console.error('Failed to play MIDI:', error);
  }
}

// Pause/Resume playback
export async function pauseResume() {
  try {
    const state = await invoke('pause_resume');
    isPaused.set(state.is_paused);
    isPlaying.set(state.is_playing);
    currentPosition.set(state.current_position);
    totalDuration.set(state.total_duration);
    if (!state.is_paused) {
      await focusGameWindow();
      delaySmartPause();
    }
  } catch (error) {
    console.error('Failed to pause/resume:', error);
  }
}

// Stop playback
export async function stopPlayback() {
  try {
    delaySmartPause();
    await invoke('stop_playback');
    isPlaying.set(false);
    isPaused.set(false);
    currentPosition.set(0);
    currentFile.set(null);
  } catch (error) {
    console.error('Failed to stop playback:', error);
  }
}

// Toggle loop mode
export async function toggleLoop() {
  const newLoopMode = !get(loopMode);
  loopMode.set(newLoopMode);

  try {
    await invoke('set_loop_mode', { enabled: newLoopMode });
    delaySmartPause();
  } catch (error) {
    console.error('Failed to set loop mode:', error);
  }
}

// Play next in playlist
export async function playNext() {
  const $playlist = get(playlist);
  const $currentIndex = get(currentIndex);

  if ($playlist.length === 0) return;

  const nextIndex = ($currentIndex + 1) % $playlist.length;
  currentIndex.set(nextIndex);

  await playMidi($playlist[nextIndex].path);
}

// Play previous in playlist
export async function playPrevious() {
  const $playlist = get(playlist);
  const $currentIndex = get(currentIndex);

  if ($playlist.length === 0) return;

  const prevIndex = ($currentIndex - 1 + $playlist.length) % $playlist.length;
  currentIndex.set(prevIndex);

  await playMidi($playlist[prevIndex].path);
}

// Toggle draggable mode
export async function toggleDraggable() {
  const newMode = !get(isDraggable);
  isDraggable.set(newMode);

  try {
    await invoke('set_interaction_mode', { interactive: newMode });
  } catch (error) {
    console.error('Failed to set interaction mode:', error);
  }
}

// Initialize event listeners
export function initializeListeners() {
  // Listen for playback progress updates
  listen('playback-progress', (event) => {
    currentPosition.set(event.payload);
  });

  // Listen for playback ended
  listen('playback-ended', async () => {
    const $playlist = get(playlist);
    const $loopMode = get(loopMode);

    if ($loopMode && $playlist.length === 1) {
      // Restart the same song
      await playMidi(get(currentFile));
    } else if ($playlist.length > 1) {
      // Play next in playlist
      await playNext();
    } else {
      // Stop playback
      isPlaying.set(false);
      currentPosition.set(0);
    }
  });

  // Update UI progress more frequently for smooth animation
  setInterval(() => {
    if (get(isPlaying) && !get(isPaused)) {
      // Increment position locally for smooth UI updates
      currentPosition.update(pos => {
        const newPos = pos + 0.1;
        const duration = get(totalDuration);
        return newPos <= duration ? newPos : duration;
      });
    }
  }, 100); // Update every 100ms for smooth progress

  // Check game focus periodically for smart pause
  setInterval(async () => {
    if (Date.now() < smartPauseCooldownUntil) {
      return;
    }

    if (get(smartPause) && get(isPlaying) && !get(isPaused)) {
      try {
        const focused = await invoke('is_game_focused');
        if (!focused) {
          await pauseResume();
        }
      } catch (error) {
        console.error('Failed to check game focus:', error);
      }
    }
  }, 1000);
}

// Utility to get store value
function get(store) {
  let value;
  store.subscribe(v => value = v)();
  return value;
}

function delaySmartPause(duration = 2000) {
  smartPauseCooldownUntil = Date.now() + duration;
}

async function focusGameWindow() {
  try {
    await invoke('focus_game_window');
    delaySmartPause();
  } catch (error) {
    console.warn('Failed to focus game window:', error);
  }
}

async function refreshPlaybackState() {
  try {
    const state = await invoke('get_playback_status');
    isPlaying.set(state.is_playing);
    isPaused.set(state.is_paused);
    loopMode.set(state.loop_mode);
    currentPosition.set(state.current_position);
    totalDuration.set(state.total_duration);
    if (state.current_file) {
      currentFile.set(state.current_file);
    }
  } catch (error) {
    console.error('Failed to refresh playback status:', error);
  }
}
