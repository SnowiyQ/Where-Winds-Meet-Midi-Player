import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

// Player state
export const isPlaying = writable(false);
export const isPaused = writable(false);
export const currentPosition = writable(0);
export const totalDuration = writable(0);
export const currentFile = writable(null);
export const loopMode = writable(false);

// Note calculation mode (default: Python/YueLyn)
export const noteMode = writable("Python");

// Key mode (21 or 36 keys)
export const keyMode = writable("Keys21");

// Modifier delay for sharps/flats in 36-key mode (ms)
export const modifierDelay = writable(2);

// Octave shift (-2 to +2)
export const octaveShift = writable(0);

// Playback speed (0.25 to 2.0, default 1.0)
export const speed = writable(1.0);

// Playlist state
export const midiFiles = writable([]);
export const playlist = writable([]);
export const currentIndex = writable(0);

// Multiple playlists support
export const savedPlaylists = writable([]);
export const activePlaylistId = writable(null);

// Favorites
export const favorites = writable([]);

// UI state
export const isDraggable = writable(true);
export const isMinimized = writable(false);
export const miniMode = writable(false);
export const smartPause = writable(false);

// Store previous window size/position for restore
let previousWindowState = null;

// Toggle mini mode with window resize
export async function toggleMiniMode() {
  const currentMiniMode = get(miniMode);
  const appWindow = getCurrentWindow();

  if (!currentMiniMode) {
    // Entering mini mode - save current state and resize
    try {
      const size = await appWindow.innerSize();
      const position = await appWindow.innerPosition();
      previousWindowState = {
        width: size.width,
        height: size.height,
        x: position.x,
        y: position.y
      };

      // Set to mini size (64x88 for the floating icon + drag handle)
      await appWindow.setMinSize(new LogicalSize(64, 88));
      await appWindow.setSize(new LogicalSize(64, 88));
    } catch (error) {
      console.error('Failed to resize window for mini mode:', error);
    }
  } else {
    // Exiting mini mode - restore previous state
    try {
      await appWindow.setMinSize(new LogicalSize(960, 540));
      if (previousWindowState) {
        await appWindow.setSize(new LogicalSize(previousWindowState.width, previousWindowState.height));
      } else {
        await appWindow.setSize(new LogicalSize(1180, 620));
      }
    } catch (error) {
      console.error('Failed to restore window from mini mode:', error);
    }
  }

  miniMode.update(v => !v);
}

let smartPauseCooldownUntil = 0;

// LocalStorage keys
const STORAGE_KEYS = {
  FAVORITES: 'wwm-favorites',
  PLAYLISTS: 'wwm-playlists',
  ACTIVE_PLAYLIST: 'wwm-active-playlist',
  NOTE_MODE: 'wwm-note-mode',
  KEY_MODE: 'wwm-key-mode',
  MODIFIER_DELAY: 'wwm-modifier-delay',
  SPEED: 'wwm-speed',
  STATS: 'wwm-stats'
};

// Stats store
export const stats = writable({
  totalPlaytime: 0,
  songsPlayed: 0,
  sessionsCount: 0,
  mostPlayed: {},
  lastPlayed: null,
  firstUsed: null,
});

// Track song play
export function trackSongPlay(filename) {
  stats.update(s => {
    s.songsPlayed++;
    s.lastPlayed = new Date().toISOString();
    s.mostPlayed[filename] = (s.mostPlayed[filename] || 0) + 1;
    if (!s.firstUsed) s.firstUsed = new Date().toISOString();
    saveStats(s);
    return s;
  });
}

// Add playtime
export function addPlaytime(seconds) {
  stats.update(s => {
    s.totalPlaytime += seconds;
    saveStats(s);
    return s;
  });
}

// Increment session
export function incrementSession() {
  stats.update(s => {
    s.sessionsCount++;
    if (!s.firstUsed) s.firstUsed = new Date().toISOString();
    saveStats(s);
    return s;
  });
}

function saveStats(s) {
  try {
    localStorage.setItem(STORAGE_KEYS.STATS, JSON.stringify(s));
  } catch (e) {
    console.error('Failed to save stats:', e);
  }
}

function loadStats() {
  try {
    const stored = localStorage.getItem(STORAGE_KEYS.STATS);
    if (stored) {
      stats.set(JSON.parse(stored));
    }
  } catch (e) {
    console.error('Failed to load stats:', e);
  }
}

// Initialize from localStorage
export async function initializeStorage() {
  // Load stats first
  loadStats();
  incrementSession();

  try {
    const storedFavorites = localStorage.getItem(STORAGE_KEYS.FAVORITES);
    if (storedFavorites) {
      favorites.set(JSON.parse(storedFavorites));
    }

    const storedPlaylists = localStorage.getItem(STORAGE_KEYS.PLAYLISTS);
    if (storedPlaylists) {
      savedPlaylists.set(JSON.parse(storedPlaylists));
    }

    const storedActivePlaylist = localStorage.getItem(STORAGE_KEYS.ACTIVE_PLAYLIST);
    if (storedActivePlaylist) {
      activePlaylistId.set(storedActivePlaylist);
    }

    // Load note mode from localStorage and sync with backend
    const storedNoteMode = localStorage.getItem(STORAGE_KEYS.NOTE_MODE);
    if (storedNoteMode) {
      noteMode.set(storedNoteMode);
      // Sync with backend
      await invoke('set_note_mode', { mode: storedNoteMode });
    }

    // Load key mode from localStorage and sync with backend
    const storedKeyMode = localStorage.getItem(STORAGE_KEYS.KEY_MODE);
    if (storedKeyMode) {
      keyMode.set(storedKeyMode);
      // Sync with backend
      await invoke('set_key_mode', { mode: storedKeyMode });
    }

    // Load modifier delay from localStorage and sync with backend
    const storedModifierDelay = localStorage.getItem(STORAGE_KEYS.MODIFIER_DELAY);
    if (storedModifierDelay) {
      const delay = parseInt(storedModifierDelay, 10);
      modifierDelay.set(delay);
      // Sync with backend
      await invoke('set_modifier_delay', { delay_ms: delay });
    }

    // Load speed from localStorage and sync with backend
    const storedSpeed = localStorage.getItem(STORAGE_KEYS.SPEED);
    if (storedSpeed) {
      const spd = parseFloat(storedSpeed);
      speed.set(spd);
      // Sync with backend
      await invoke('set_speed', { speed: spd });
    }
  } catch (error) {
    console.error('Failed to load from localStorage:', error);
  }
}

// Save favorites to localStorage
function saveFavorites(favs) {
  try {
    localStorage.setItem(STORAGE_KEYS.FAVORITES, JSON.stringify(favs));
  } catch (error) {
    console.error('Failed to save favorites:', error);
  }
}

// Save playlists to localStorage
function savePlaylists(lists) {
  try {
    localStorage.setItem(STORAGE_KEYS.PLAYLISTS, JSON.stringify(lists));
  } catch (error) {
    console.error('Failed to save playlists:', error);
  }
}

// Favorites operations
export function toggleFavorite(file) {
  favorites.update(favs => {
    const exists = favs.find(f => f.path === file.path);
    let newFavs;
    if (exists) {
      newFavs = favs.filter(f => f.path !== file.path);
    } else {
      newFavs = [...favs, file];
    }
    saveFavorites(newFavs);
    return newFavs;
  });
}

export function isFavorite(path) {
  let result = false;
  favorites.subscribe(favs => {
    result = favs.some(f => f.path === path);
  })();
  return result;
}

// Playlist operations
export function createPlaylist(name) {
  const id = Date.now().toString();
  const newPlaylist = {
    id,
    name,
    tracks: [],
    createdAt: new Date().toISOString()
  };

  savedPlaylists.update(lists => {
    const newLists = [...lists, newPlaylist];
    savePlaylists(newLists);
    return newLists;
  });

  return id;
}

export function deletePlaylist(id) {
  savedPlaylists.update(lists => {
    const newLists = lists.filter(p => p.id !== id);
    savePlaylists(newLists);
    return newLists;
  });

  // If active playlist was deleted, clear it
  const currentActive = get(activePlaylistId);
  if (currentActive === id) {
    activePlaylistId.set(null);
  }
}

export function renamePlaylist(id, newName) {
  savedPlaylists.update(lists => {
    const newLists = lists.map(p =>
      p.id === id ? { ...p, name: newName } : p
    );
    savePlaylists(newLists);
    return newLists;
  });
}

export function addToSavedPlaylist(playlistId, file) {
  savedPlaylists.update(lists => {
    const newLists = lists.map(p => {
      if (p.id === playlistId) {
        // Check for duplicate
        if (!p.tracks.find(t => t.path === file.path)) {
          return { ...p, tracks: [...p.tracks, file] };
        }
      }
      return p;
    });
    savePlaylists(newLists);
    return newLists;
  });
}

export function removeFromSavedPlaylist(playlistId, filePath) {
  savedPlaylists.update(lists => {
    const newLists = lists.map(p => {
      if (p.id === playlistId) {
        return { ...p, tracks: p.tracks.filter(t => t.path !== filePath) };
      }
      return p;
    });
    savePlaylists(newLists);
    return newLists;
  });
}

export function reorderSavedPlaylist(playlistId, fromIndex, toIndex) {
  savedPlaylists.update(lists => {
    const newLists = lists.map(p => {
      if (p.id === playlistId) {
        const tracks = [...p.tracks];
        const [item] = tracks.splice(fromIndex, 1);
        tracks.splice(toIndex, 0, item);
        return { ...p, tracks };
      }
      return p;
    });
    savePlaylists(newLists);
    return newLists;
  });
}

export function reorderPlaylists(fromIndex, toIndex) {
  savedPlaylists.update(lists => {
    const newLists = [...lists];
    const [item] = newLists.splice(fromIndex, 1);
    newLists.splice(toIndex, 0, item);
    savePlaylists(newLists);
    return newLists;
  });
}

export async function loadPlaylistToQueue(playlistId, autoPlay = true) {
  const lists = get(savedPlaylists);
  const targetPlaylist = lists.find(p => p.id === playlistId);
  if (targetPlaylist && targetPlaylist.tracks.length > 0) {
    playlist.set([...targetPlaylist.tracks]);
    currentIndex.set(0);
    activePlaylistId.set(playlistId);
    localStorage.setItem(STORAGE_KEYS.ACTIVE_PLAYLIST, playlistId);

    // Auto-play first track
    if (autoPlay) {
      await playMidi(targetPlaylist.tracks[0].path);
    }
  }
}

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
  } catch (error) {
    console.error('Failed to load MIDI files:', error);
  }
}

// Import a MIDI file to album folder
export async function importMidiFile(sourcePath) {
  try {
    const newFile = await invoke('import_midi_file', { sourcePath });
    // Add to midiFiles store
    midiFiles.update(files => [...files, newFile]);
    return { success: true, file: newFile };
  } catch (error) {
    console.error('Failed to import MIDI file:', error);
    return { success: false, error: error.toString() };
  }
}

// Play a MIDI file
export async function playMidi(path) {
  try {
    delaySmartPause();

    // Reset state immediately before playing
    currentPosition.set(0);
    isPlaying.set(false);
    isPaused.set(false);

    await invoke('play_midi', { path });

    // Small delay to let backend initialize
    await new Promise(resolve => setTimeout(resolve, 50));

    // Focus is now handled in the backend after playback starts
    await refreshPlaybackState();
    isPlaying.set(true);
    isPaused.set(false);
    currentFile.set(path);

    // Track stats
    const filename = path.split(/[\\/]/).pop() || path;
    trackSongPlay(filename);
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

// Seek to a specific position (in seconds)
export async function seekTo(position) {
  try {
    const duration = get(totalDuration);
    const clamped = Math.max(0, Math.min(position, duration));
    currentPosition.set(clamped);
    await invoke('seek', { position: clamped });
    delaySmartPause();
  } catch (error) {
    console.error('Failed to seek:', error);
  }
}

// Set note calculation mode (Default or Detailed)
export async function setNoteMode(mode) {
  try {
    await invoke('set_note_mode', { mode });
    noteMode.set(mode);
    localStorage.setItem(STORAGE_KEYS.NOTE_MODE, mode);
    console.log(`Note mode set to: ${mode}`);
  } catch (error) {
    console.error('Failed to set note mode:', error);
  }
}

// Set octave shift (-2 to +2)
export async function setOctaveShift(shift) {
  try {
    const clamped = Math.max(-2, Math.min(2, shift));
    await invoke('set_octave_shift', { shift: clamped });
    octaveShift.set(clamped);
    console.log(`Octave shift set to: ${clamped}`);
  } catch (error) {
    console.error('Failed to set octave shift:', error);
  }
}

// Set key mode (Keys21 or Keys36)
export async function setKeyMode(mode) {
  try {
    await invoke('set_key_mode', { mode });
    keyMode.set(mode);
    localStorage.setItem(STORAGE_KEYS.KEY_MODE, mode);
    console.log(`Key mode set to: ${mode}`);
  } catch (error) {
    console.error('Failed to set key mode:', error);
  }
}

// Set modifier delay for sharps/flats (ms)
export async function setModifierDelay(delayMs) {
  const clamped = Math.max(0, Math.min(50, delayMs));
  // Update store immediately for responsive UI
  modifierDelay.set(clamped);
  localStorage.setItem(STORAGE_KEYS.MODIFIER_DELAY, clamped.toString());
  try {
    await invoke('set_modifier_delay', { delay_ms: clamped });
    console.log(`Modifier delay set to: ${clamped}ms`);
  } catch (error) {
    console.error('Failed to set modifier delay:', error);
  }
}

// Set playback speed (0.25 to 2.0)
export async function setSpeed(newSpeed) {
  const clamped = Math.max(0.25, Math.min(2.0, newSpeed));
  // Update store immediately for responsive UI
  speed.set(clamped);
  localStorage.setItem(STORAGE_KEYS.SPEED, clamped.toString());
  try {
    await invoke('set_speed', { speed: clamped });
    console.log(`Speed set to: ${clamped}x`);
  } catch (error) {
    console.error('Failed to set speed:', error);
  }
}

// Test all 21 keys
export async function testAllKeys() {
  try {
    await invoke('test_all_keys');
    console.log('21-key test complete');
  } catch (error) {
    console.error('Failed to test all keys:', error);
  }
}

// Test all 36 keys (including modifiers)
export async function testAllKeys36() {
  try {
    await invoke('test_all_keys_36');
    console.log('36-key test complete');
  } catch (error) {
    console.error('Failed to test 36 keys:', error);
  }
}

// Play next in playlist
export async function playNext() {
  const $playlist = get(playlist);
  const $currentIndex = get(currentIndex);

  if ($playlist.length === 0) return;

  const nextIndex = ($currentIndex + 1) % $playlist.length;
  currentIndex.set(nextIndex);

  // Reset position immediately before starting new track
  currentPosition.set(0);
  totalDuration.set(0);

  await playMidi($playlist[nextIndex].path);
}

// Play previous in playlist
export async function playPrevious() {
  const $playlist = get(playlist);
  const $currentIndex = get(currentIndex);

  if ($playlist.length === 0) return;

  const prevIndex = ($currentIndex - 1 + $playlist.length) % $playlist.length;
  currentIndex.set(prevIndex);

  // Reset position immediately before starting new track
  currentPosition.set(0);
  totalDuration.set(0);

  await playMidi($playlist[prevIndex].path);
}

// Add to queue and optionally play
export function addToQueue(file, playNow = false) {
  playlist.update(list => {
    // Allow duplicates in queue (unlike library playlists)
    const newList = [...list, file];
    if (playNow && list.length === 0) {
      // If queue was empty, play the first item
      setTimeout(() => playMidi(file.path), 0);
    }
    return newList;
  });
}

// Reorder queue
export function reorderQueue(fromIndex, toIndex) {
  playlist.update(list => {
    const items = [...list];
    const [item] = items.splice(fromIndex, 1);
    items.splice(toIndex, 0, item);

    // Update currentIndex if needed
    const $currentIndex = get(currentIndex);
    if (fromIndex === $currentIndex) {
      currentIndex.set(toIndex);
    } else if (fromIndex < $currentIndex && toIndex >= $currentIndex) {
      currentIndex.set($currentIndex - 1);
    } else if (fromIndex > $currentIndex && toIndex <= $currentIndex) {
      currentIndex.set($currentIndex + 1);
    }

    return items;
  });
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
  // Initialize storage first
  initializeStorage();

  // Listen for playback progress updates from backend (single source of truth)
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
    if (state.note_mode) {
      noteMode.set(state.note_mode);
    }
    if (state.key_mode) {
      keyMode.set(state.key_mode);
    }
    if (state.octave_shift !== undefined) {
      octaveShift.set(state.octave_shift);
    }
    if (state.speed !== undefined) {
      speed.set(state.speed);
    }
  } catch (error) {
    console.error('Failed to refresh playback status:', error);
  }
}
