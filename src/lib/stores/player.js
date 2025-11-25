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

// Multiple playlists support
export const savedPlaylists = writable([]);
export const activePlaylistId = writable(null);

// Favorites
export const favorites = writable([]);

// UI state
export const isDraggable = writable(true);
export const isMinimized = writable(false);
export const smartPause = writable(true);

let smartPauseCooldownUntil = 0;

// LocalStorage keys
const STORAGE_KEYS = {
  FAVORITES: 'wwm-favorites',
  PLAYLISTS: 'wwm-playlists',
  ACTIVE_PLAYLIST: 'wwm-active-playlist'
};

// Initialize from localStorage
export function initializeStorage() {
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
  } catch (error) {
    console.error('Failed to refresh playback status:', error);
  }
}
