<script>
  import { onMount } from "svelte";
  import { fade, fly, scale } from "svelte/transition";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "./lib/tauri/core-proxy.js";
  import { onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";

  // i18n (must be imported early)
  import "./lib/i18n";
  import { t } from "svelte-i18n";
  import { languages, currentLanguage, setLanguage, currentLanguageInfo, initUserLocales } from "./lib/i18n";

  // Current version
  import { APP_VERSION, APP_FLAVOR } from "./lib/version.js";
  import { recordingKeybind, isImportingFiles } from "./lib/stores/player.js";

  // Game window detection
  let gameFound = false;
  let checkInterval;

  // Always on top toggle
  let isAlwaysOnTop = true; // Default from tauri.conf.json

  async function loadAlwaysOnTop() {
    try {
      const saved = await invoke('get_always_on_top');
      isAlwaysOnTop = saved;
      const appWindow = getCurrentWindow();
      await appWindow.setAlwaysOnTop(isAlwaysOnTop);
    } catch (e) {
      console.error('Failed to load always on top setting:', e);
    }
  }

  async function toggleAlwaysOnTop() {
    try {
      const appWindow = getCurrentWindow();
      isAlwaysOnTop = !isAlwaysOnTop;
      await appWindow.setAlwaysOnTop(isAlwaysOnTop);
      await invoke('save_always_on_top', { enabled: isAlwaysOnTop });
    } catch (e) {
      console.error('Failed to toggle always on top:', e);
    }
  }

  // Custom keybindings
  let keybindings = {
    pause_resume: "F9",
    stop: "F12",
    previous: "F10",
    next: "F11",
    mode_prev: "[",
    mode_next: "]",
    toggle_mini: "Insert"
  };

  // Convert key name to event.code for language-independent detection
  function keyToCode(key) {
    const upper = key.toUpperCase();
    // Function keys
    if (/^F\d{1,2}$/.test(upper)) return upper;
    // Letters
    if (/^[A-Z]$/.test(upper)) return `Key${upper}`;
    // Numbers
    if (/^[0-9]$/.test(key)) return `Digit${key}`;
    // Special keys
    const codeMap = {
      '[': 'BracketLeft', ']': 'BracketRight',
      '`': 'Backquote', '-': 'Minus', '=': 'Equal',
      '\\': 'Backslash', ';': 'Semicolon', "'": 'Quote',
      ',': 'Comma', '.': 'Period', '/': 'Slash',
      'INSERT': 'Insert', 'DELETE': 'Delete',
      'HOME': 'Home', 'END': 'End',
      'PAGEUP': 'PageUp', 'PAGEDOWN': 'PageDown',
      'UP': 'ArrowUp', 'DOWN': 'ArrowDown',
      'LEFT': 'ArrowLeft', 'RIGHT': 'ArrowRight',
      'SCROLLLOCK': 'ScrollLock', 'PAUSE': 'Pause',
      'NUMLOCK': 'NumLock', 'PRINTSCREEN': 'PrintScreen',
    };
    return codeMap[upper] || codeMap[key] || key;
  }

  async function loadKeybindings() {
    try {
      keybindings = await invoke('cmd_get_keybindings');
    } catch (e) {
      console.error("Failed to load keybindings:", e);
    }
  }

  // Update check
  let updateAvailable = null; // { version, download_url, release_url, file_name }
  let updateStatus = "idle"; // idle, checking, downloading, installing, error
  let updateError = "";
  let downloadedPath = "";

  async function checkForUpdates() {
    try {
      updateStatus = "checking";
      const result = await invoke('check_for_update', { currentVersion: APP_VERSION });
      if (result) {
        updateAvailable = result;
      }
      updateStatus = "idle";
    } catch (e) {
      console.log('Update check failed:', e);
      updateStatus = "idle";
    }
  }

  async function downloadUpdate() {
    if (!updateAvailable) return;
    try {
      updateStatus = "downloading";
      updateError = "";
      downloadedPath = await invoke('download_update', {
        downloadUrl: updateAvailable.download_url,
        fileName: updateAvailable.file_name
      });
      updateStatus = "downloaded";
    } catch (e) {
      updateStatus = "error";
      updateError = e.toString();
    }
  }

  async function installUpdate() {
    if (!downloadedPath) return;
    try {
      updateStatus = "installing";
      await invoke('install_update', { zipPath: downloadedPath });
    } catch (e) {
      updateStatus = "error";
      updateError = e.toString();
    }
  }

  async function checkGameWindow() {
    try {
      gameFound = await invoke('is_game_window_found');
    } catch {
      gameFound = false;
    }
  }

  // Check every 2 seconds
  checkInterval = setInterval(checkGameWindow, 2000);
  checkGameWindow(); // Initial check

  // Window position saving
  let savePositionInterval;
  let lastSavedPosition = null;

  async function saveWindowPosition() {
    try {
      const appWindow = getCurrentWindow();
      const pos = await appWindow.outerPosition();
      const size = await appWindow.outerSize();

      // Only save if position changed
      const newPos = `${pos.x},${pos.y},${size.width},${size.height}`;
      if (newPos !== lastSavedPosition) {
        lastSavedPosition = newPos;
        await invoke('save_window_position', {
          x: pos.x,
          y: pos.y,
          width: size.width,
          height: size.height
        });
      }
    } catch (e) {
      console.error('Failed to save window position:', e);
    }
  }

  async function loadWindowPosition() {
    try {
      const saved = await invoke('get_window_position');
      if (saved) {
        const appWindow = getCurrentWindow();
        await appWindow.setPosition(new PhysicalPosition(saved.x, saved.y));
        await appWindow.setSize(new PhysicalSize(saved.width, saved.height));
      }
    } catch (e) {
      console.error('Failed to load window position:', e);
    }
  }

  async function closeApp() {
    try {
      showCloseConfirmModal = false;
      await saveWindowPosition();
      await invoke('cmd_exit_app');
    } catch (e) {
      console.error('Failed to close app:', e);
    }
  }

  onDestroy(() => {
    if (checkInterval) clearInterval(checkInterval);
    if (savePositionInterval) clearInterval(savePositionInterval);
    saveWindowPosition(); // Save on destroy
  });
  import Icon from "@iconify/svelte";
  import appIcon from "./icon.png";

  import Header from "./lib/components/Header.svelte";
  import MidiFileList from "./lib/components/MidiFileList.svelte";
  import PlaybackControls from "./lib/components/PlaybackControls.svelte";
  import Timeline from "./lib/components/Timeline.svelte";
  import PlaylistManager from "./lib/components/PlaylistManager.svelte";
  import FavoritesView from "./lib/components/FavoritesView.svelte";
  import SavedPlaylistsView from "./lib/components/SavedPlaylistsView.svelte";
  import SettingsView from "./lib/components/SettingsView.svelte";
  import StatsView from "./lib/components/StatsView.svelte";
  import LivePlayView from "./lib/components/LivePlayView.svelte";
  import Visualizer from "./lib/components/Visualizer.svelte";
  import BandMode from "./lib/components/BandMode.svelte";
  import LibraryShare from "./lib/components/LibraryShare.svelte";

  import { bandStatus, connectedPeers, bandSongSelectMode, cancelBandSongSelect, bandSelectedSong } from "./lib/stores/band.js";
  import { libraryConnected, onlinePeers, initLibrary, shareNotification, libraryEnabled } from "./lib/stores/library.js";

  // DEV: Simulate share notification for testing
  function simulateShareNotification() {
    shareNotification.set({
      songName: "Test Song - Example MIDI File.mid",
      peerName: "HappyMusician42",
      timestamp: Date.now()
    });
  }

  import {
    loadMidiFiles,
    shouldShowLibraryWarning,
    loadInitialBatch,
    initializeListeners,
    isMinimized,
    isDraggable,
    miniMode,
    toggleMiniMode,
    currentFile,
    playlist,
    favorites,
    toggleFavorite,
    midiFiles,
    savedPlaylists,
    smartPause,
    loopMode,
    isPlaying,
    isPaused,
    pauseResume,
    stopPlayback,
    playNext,
    playPrevious,
    toggleLoop,
    toggleDraggable,
    noteMode,
    setNoteMode,
    keyMode,
    setKeyMode,
    octaveShift,
    setOctaveShift,
    speed,
    setSpeed,
    availableTracks,
    selectedTrackId,
    loadTracksForFile,
    setSelectedTrack,
    libraryPlayMode,
    libraryPlayShuffle,
    libraryPlayIndex,
    exitLibraryPlayMode,
  } from "./lib/stores/player.js";


  // Note mode options for quick selector (reactive for i18n)
  $: noteModeOptionsList = [
    { id: "Python", title: $t("noteMode.yuelyn"), short: "YL", icon: "mdi:heart", desc: $t("noteMode.yuelynDesc"), rmd21: true },
    { id: "Closest", title: $t("noteMode.closest"), short: "CLS", icon: "mdi:target", desc: $t("noteMode.closestDesc") },
    { id: "Wide", title: $t("noteMode.wide"), short: "WDE", icon: "mdi:arrow-expand-horizontal", desc: $t("noteMode.wideDesc") },
    { id: "Sharps", title: $t("noteMode.sharps"), short: "SHP", icon: "mdi:music-accidental-sharp", desc: $t("noteMode.sharpsDesc"), rmd36: true },
    { id: "Quantize", title: $t("noteMode.quantize"), short: "QNT", icon: "mdi:grid", desc: $t("noteMode.quantizeDesc") },
    { id: "TransposeOnly", title: $t("noteMode.transposeOnly"), short: "TRP", icon: "mdi:arrow-up-down", desc: $t("noteMode.transposeOnlyDesc") },
    { id: "Pentatonic", title: $t("noteMode.pentatonic"), short: "PEN", icon: "mdi:music", desc: $t("noteMode.pentatonicDesc") },
    { id: "Chromatic", title: $t("noteMode.chromatic"), short: "CHR", icon: "mdi:piano", desc: $t("noteMode.chromaticDesc") },
    { id: "Raw", title: $t("noteMode.raw"), short: "RAW", icon: "mdi:code-braces", desc: $t("noteMode.rawDesc") },
  ];

  // Reactive: show RMD based on key mode
  $: noteModeOptions = noteModeOptionsList.map(m => ({
    ...m,
    isRmd: ($keyMode === 'Keys36' && m.rmd36) || ($keyMode === 'Keys21' && m.rmd21)
  }));

  let showModeMenu = false;
  let showSpeedMenu = false;
  let showTrackMenu = false;
  let showLanguageMenu = false;
  let showVisualizer = false;
  let showUpdateModal = false;
  let showLargeLibraryModal = false;
  let showCloseConfirmModal = false;
  let largeLibraryCount = 0;

  // Load tracks when current file changes
  $: if ($currentFile) {
    loadTracksForFile($currentFile);
  } else {
    setSelectedTrack(null);
  }

  async function selectTrack(trackId) {
    showTrackMenu = false;
    await setSelectedTrack(trackId);
  }

  // Speed presets
  const speedOptions = [
    { value: 0.25, label: "0.25x" },
    { value: 0.5, label: "0.5x" },
    { value: 0.75, label: "0.75x" },
    { value: 1.0, label: "1.0x" },
    { value: 1.25, label: "1.25x" },
    { value: 1.5, label: "1.5x" },
    { value: 1.75, label: "1.75x" },
    { value: 2.0, label: "2.0x" },
  ];

  function selectSpeed(value) {
    setSpeed(value);
    showSpeedMenu = false;
  }

  function nextNoteMode() {
    const currentIndex = noteModeOptions.findIndex(m => m.id === $noteMode);
    const nextIndex = (currentIndex + 1) % noteModeOptions.length;
    setNoteMode(noteModeOptions[nextIndex].id);
  }

  function prevNoteMode() {
    const currentIndex = noteModeOptions.findIndex(m => m.id === $noteMode);
    const prevIndex = (currentIndex - 1 + noteModeOptions.length) % noteModeOptions.length;
    setNoteMode(noteModeOptions[prevIndex].id);
  }

  function selectNoteMode(modeId) {
    setNoteMode(modeId);
    showModeMenu = false;
  }

  let activeView = "library"; // "library", "queue", "favorites", "playlists"

  // Reactive badge counts
  $: queueCount = $playlist.length;
  $: favoritesCount = $favorites.length;
  $: playlistsCount = $savedPlaylists.length;

  let sidebarTab = "music"; // "music", "online", or "app"

  // Band mode connected peers count (excluding self)
  $: bandPeersCount = $bandStatus === 'connected' ? $connectedPeers.length : 0;
  $: libraryPeersCount = $libraryConnected ? $onlinePeers : 0;

  // Auto-dismiss share notification after 5 seconds
  let shareNotificationTimeout;
  $: if ($shareNotification) {
    if (shareNotificationTimeout) clearTimeout(shareNotificationTimeout);
    shareNotificationTimeout = setTimeout(() => {
      shareNotification.set(null);
    }, 5000);
  }

  function dismissShareNotification() {
    if (shareNotificationTimeout) clearTimeout(shareNotificationTimeout);
    shareNotification.set(null);
  }

  $: musicNavItems = [
    { id: "library", icon: "mdi:library-music", label: $t("nav.library"), badge: 0 },
    { id: "queue", icon: "mdi:playlist-play", label: $t("nav.queue"), badge: queueCount },
    { id: "favorites", icon: "mdi:heart", label: $t("nav.favorites"), badge: favoritesCount },
    { id: "playlists", icon: "mdi:folder-music", label: $t("nav.playlists"), badge: playlistsCount },
  ];

  $: onlineNavItems = [
    { id: "band", icon: "mdi:account-group", label: $t("nav.band"), badge: bandPeersCount, status: $bandStatus },
    { id: "share", icon: "mdi:earth", label: $t("nav.share"), badge: libraryPeersCount, status: $libraryConnected ? 'connected' : 'disconnected' },
  ];

  $: appNavItems = [
    { id: "live", icon: "mdi:piano", label: $t("nav.livePlay"), badge: 0 },
    { id: "settings", icon: "mdi:cog", label: $t("nav.settings"), badge: 0 },
    { id: "stats", icon: "mdi:chart-bar", label: $t("nav.stats"), badge: 0 },
  ];

  $: navItems = sidebarTab === "music" ? musicNavItems : sidebarTab === "online" ? onlineNavItems : appNavItems;

  // Reactive shortcuts based on custom keybindings
  $: shortcuts = [
    { action: $t("controls.playPause"), key: keybindings.pause_resume },
    { action: $t("player.stop"), key: `${keybindings.stop} / End` },
    { action: $t("player.previous"), key: keybindings.previous },
    { action: $t("player.next"), key: keybindings.next },
    { action: $t("controls.mode"), key: `${keybindings.mode_prev} / ${keybindings.mode_next}` },
  ];

  // Check if current song is favorited
  $: currentFileIsFavorite = $currentFile && $favorites.some(f => f.path === $currentFile);
  $: currentFileData = $currentFile ? $midiFiles.find(f => f.path === $currentFile) : null;

  function toggleCurrentFavorite() {
    if (currentFileData) {
      toggleFavorite(currentFileData);
    }
  }

  function handleBandSelectSong() {
    activeView = "library";
    sidebarTab = "music";
  }

  function handleCancelBandSelect() {
    cancelBandSongSelect();
  }

  // Watch for band song selection completion - navigate back to band
  let prevBandSelectMode = false;
  $: {
    // When selection mode turns off (song was selected), go back to band
    if (prevBandSelectMode && !$bandSongSelectMode && $bandSelectedSong) {
      activeView = "band";
      sidebarTab = "online";
    }
    prevBandSelectMode = $bandSongSelectMode;
  }

  onMount(async () => {
    await loadWindowPosition(); // Restore window position
    await loadAlwaysOnTop(); // Restore always on top setting
    initUserLocales(); // Initialize user locale files (async, don't await)

    // Check library size AND cache status before loading
    const { needsWarning, isLarge, count, isCached } = await shouldShowLibraryWarning();

    if (needsWarning) {
      // Large library with uncached files - show warning (first time only)
      largeLibraryCount = count;
      showLargeLibraryModal = true;
      // Don't auto-load, wait for user choice
    } else if (isLarge && isCached) {
      // Large library but cached - load all (fast)
      await loadMidiFiles();
    } else if (isLarge) {
      // Large but somehow not cached and no warning? Load batch to be safe
      await loadInitialBatch();
    } else {
      // Small library - just load all
      await loadMidiFiles();
    }

    await loadKeybindings(); // Load custom keybindings
    initializeListeners();
    initLibrary(); // Initialize library sharing (auto-connects if was enabled)
    checkForUpdates(); // Check for updates on startup
    setInterval(checkForUpdates, 30 * 60 * 1000); // Check every 30 minutes

    // Save window position every 5 seconds (only if changed)
    savePositionInterval = setInterval(saveWindowPosition, 5000);

    // Listen for open-update-modal event from SettingsView
    const handleOpenUpdateModal = () => {
      if (updateAvailable) {
        showUpdateModal = true;
      }
    };
    window.addEventListener('open-update-modal', handleOpenUpdateModal);

    // Listen for keybindings changes from SettingsView
    const handleKeybindingsChanged = (event) => {
      keybindings = event.detail;
    };
    window.addEventListener('keybindings-changed', handleKeybindingsChanged);

    // Listen for global shortcut events from Rust backend
    const unlisten = await listen("global-shortcut", async (event) => {
      const action = event.payload;
      console.log(`Global shortcut received: ${action}`);

      switch (action) {
        case "pause_resume":
          await pauseResume();
          break;
        case "stop":
          await stopPlayback();
          break;
        case "previous":
          await playPrevious();
          break;
        case "next":
          await playNext();
          break;
        case "toggle_loop":
          await toggleLoop();
          break;
        case "mode_prev":
          prevNoteMode();
          break;
        case "mode_next":
          nextNoteMode();
          break;
        case "toggle_mini":
          toggleMiniMode();
          break;
      }
    });

    return () => {
      unlisten();
      window.removeEventListener('open-update-modal', handleOpenUpdateModal);
      window.removeEventListener('keybindings-changed', handleKeybindingsChanged);
    };
  });

  const filename = (path, fallback) => {
    if (!path) return fallback;
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1] || path;
  };

  // Handle keyboard shortcuts when app is focused
  // Uses event.code for language-independent physical key detection
  async function handleKeydown(event) {
    // Skip if recording keybind
    if ($recordingKeybind) return;

    // Skip if user is typing in an input
    if (event.target.tagName === 'INPUT' || event.target.tagName === 'TEXTAREA') {
      return;
    }

    const code = event.code;

    // Check against custom keybindings (converted to event.code format)
    if (code === keyToCode(keybindings.pause_resume)) {
      event.preventDefault();
      await pauseResume();
    } else if (code === keyToCode(keybindings.stop) || code === 'End') {
      event.preventDefault();
      await stopPlayback();
    } else if (code === keyToCode(keybindings.previous)) {
      event.preventDefault();
      await playPrevious();
    } else if (code === keyToCode(keybindings.next)) {
      event.preventDefault();
      await playNext();
    } else if (code === keyToCode(keybindings.mode_prev)) {
      event.preventDefault();
      prevNoteMode();
    } else if (code === keyToCode(keybindings.mode_next)) {
      event.preventDefault();
      nextNoteMode();
    } else if (code === keyToCode(keybindings.toggle_mini)) {
      event.preventDefault();
      toggleMiniMode();
    }
  }

  // Toggle body/html background and overflow based on mini mode
  $: {
    if (typeof document !== "undefined") {
      if ($miniMode) {
        document.body.style.background = "transparent";
        document.body.style.overflow = "hidden";
        document.body.style.border = "none";
        document.body.style.borderRadius = "0";
        document.documentElement.style.background = "transparent";
        document.documentElement.style.overflow = "hidden";
      } else {
        document.body.style.background = "";
        document.body.style.overflow = "";
        document.documentElement.style.background = "";
        document.documentElement.style.overflow = "";
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $miniMode}
  <!-- Mini Mode - Container with drag handle -->
  <div class="flex flex-col items-center select-none">
    <!-- Drag handle above the icon (same style as main app) -->
    <div
      class="drag-handle flex bg-[#18181893] items-center justify-center cursor-move hover:opacity-80 transition-colors group mb-0.5 px-3 rounded"
      title="Drag to move"
    >
      <Icon
        icon="mdi:drag-horizontal"
        class="w-5 h-5 text-white/20 group-hover:text-white/40 transition-colors"
      />
    </div>

    <!-- Clickable Icon -->
    <button
      class="w-14 h-14 rounded-2xl bg-[#18181893] border border-white/10 shadow-2xl overflow-hidden relative flex items-center justify-center cursor-pointer active:scale-95 transition-transform"
      onclick={toggleMiniMode}
      title="Click to expand"
    >
      <!-- Playing indicator ring -->
      {#if $isPlaying && !$isPaused}
        <div
          class="absolute inset-0 rounded-2xl border-2 border-[#1db954] animate-pulse pointer-events-none"
        ></div>
      {/if}

      <!-- App Icon -->
      <img
        src={appIcon}
        alt="App Icon"
        class="w-10 h-10 rounded-lg pointer-events-none"
      />
    </button>
  </div>
{:else}
  <main class="">
    <div
      class="h-screen w-full flex flex-col overflow-hidden rounded-md select-none {$isDraggable
        ? ''
        : 'pointer-events-none'}"
    >
      {#if !$isMinimized}
        <!-- Spotify-style layout -->
        <div class="flex flex-1 min-h-0 overflow-hidden">
          <!-- Sidebar -->
          <aside
            class="spotify-sidebar w-56 flex flex-col p-4 gap-2 no-drag border-r border-white/5"
          >
            <!-- Drag Handle with Window Controls -->
            <div
              class="flex items-center gap-1 py-2 -mx-4 -mt-4 mb-2 px-2"
            >
              <div
                class="drag-handle flex-1 flex items-center justify-center cursor-move hover:bg-white/5 transition-colors group py-1 rounded"
                title="Drag to move window"
              >
                <Icon
                  icon="mdi:drag-horizontal"
                  class="w-6 h-6 text-white/20 group-hover:text-white/40 transition-colors"
                />
              </div>
              <div class="flex items-center gap-0.5">
                <button
                  class="w-7 h-7 flex items-center justify-center rounded-md transition-all {isAlwaysOnTop ? 'text-[#1db954] bg-[#1db954]/10' : 'text-white/40 hover:text-white hover:bg-white/10'}"
                  onclick={toggleAlwaysOnTop}
                  title={isAlwaysOnTop ? 'Disable always on top' : 'Enable always on top'}
                >
                  <Icon icon={isAlwaysOnTop ? "mdi:pin" : "mdi:pin-off"} class="w-3.5 h-3.5" />
                </button>
                <button
                  class="w-7 h-7 flex items-center justify-center rounded-md text-white/40 hover:text-white hover:bg-white/10 transition-all"
                  onclick={toggleMiniMode}
                  title="Minimize to floating icon"
                >
                  <Icon icon="mdi:minus" class="w-4 h-4" />
                </button>
                <button
                  class="w-7 h-7 flex items-center justify-center rounded-md text-white/40 hover:text-red-400 hover:bg-red-400/10 transition-all"
                  onclick={() => showCloseConfirmModal = true}
                  title="Close application"
                >
                  <Icon icon="mdi:close" class="w-4 h-4" />
                </button>
              </div>
            </div>

            <!-- Logo / Title -->
            <!-- <div class="px-3 py-2 mb-2 -mt-2"> -->
            <!-- <h1 class="text-lg font-bold text-white/90">WWM Overlay</h1> -->
            <!-- <p class="text-xs text-white/40">By YueLyn</p> -->
            <!-- </div> -->

            <!-- Sidebar Tabs -->
            <div class="flex gap-1 mb-2">
              <button
                class="flex-1 py-1.5 px-1 rounded-lg text-xs font-medium transition-all {sidebarTab === 'music' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                onclick={() => { sidebarTab = 'music'; activeView = 'library'; }}
              >
                <Icon icon="mdi:music" class="w-3.5 h-3.5 inline" />
              </button>
              <button
                class="flex-1 py-1.5 px-1 rounded-lg text-xs font-medium transition-all relative {sidebarTab === 'online' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                onclick={() => { sidebarTab = 'online'; activeView = 'band'; }}
              >
                <Icon icon="mdi:access-point" class="w-3.5 h-3.5 inline" />
                {#if $bandStatus === 'connected'}
                  <div class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-green-500"></div>
                {/if}
              </button>
              <button
                class="flex-1 py-1.5 px-1 rounded-lg text-xs font-medium transition-all {sidebarTab === 'app' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                onclick={() => { sidebarTab = 'app'; activeView = 'settings'; }}
              >
                <Icon icon="mdi:cog" class="w-3.5 h-3.5 inline" />
              </button>
            </div>

            <!-- Navigation -->
            <nav class="flex flex-col gap-1">
              {#each navItems as item}
                <button
                  class="nav-item group flex items-center gap-3 px-3 py-2.5 rounded-lg text-left transition-all duration-200 {activeView ===
                  item.id
                    ? 'bg-white/10 text-white'
                    : 'text-white/60 hover:text-white hover:bg-white/5'}"
                  onclick={() => (activeView = item.id)}
                >
                  <div class="relative">
                    <Icon
                      icon={item.icon}
                      class="w-5 h-5 transition-transform duration-200 {activeView ===
                      item.id
                        ? 'scale-110'
                        : 'group-hover:scale-105'}"
                    />
                    {#if activeView === item.id}
                      <div
                        class="absolute -left-3 top-1/2 -translate-y-1/2 w-1 h-4 bg-[#1db954] rounded-r"
                        in:fly={{ x: -10, duration: 200 }}
                      ></div>
                    {/if}
                  </div>
                  <span class="font-medium text-sm">{item.label}</span>
                  {#if item.status}
                    <div class="ml-auto w-2 h-2 rounded-full {item.status === 'connected' ? 'bg-green-500' : item.status === 'connecting' ? 'bg-yellow-500 animate-pulse' : 'bg-white/20'}"></div>
                  {:else if item.badge > 0}
                    <span
                      class="ml-auto text-xs px-2 py-0.5 rounded-full bg-white/10 text-white/60"
                      in:fade={{ duration: 150 }}
                    >
                      {item.badge}
                    </span>
                  {/if}
                </button>
              {/each}
            </nav>

            <!-- Spacer -->
            <div class="flex-1"></div>

            <!-- Refresh Button -->
            <!-- <button
              class="flex items-center gap-2 px-3 py-2 rounded-lg text-white/60 hover:text-white hover:bg-white/5 transition-all w-full"
              onclick={loadMidiFiles}
              title="Refresh library"
            >
              <Icon icon="mdi:refresh" class="w-5 h-5" />
              <span class="font-medium text-sm">Refresh</span>
            </button> -->

            <p class="text-xs text-white/40 px-3">By YueLyn · v{APP_VERSION}{APP_FLAVOR ? `(${APP_FLAVOR})` : ''}</p>

            <!-- Ko-fi Support -->
            <button
              onclick={() => invoke('open_url', { url: 'https://ko-fi.com/snowiy' })}
              class="flex items-center gap-2 px-3 py-2 rounded-lg text-white/60 hover:text-[#ff5e5b] hover:bg-[#ff5e5b]/10 transition-all w-full mt-1"
              title={$t("common.supportOnKofi")}
            >
              <Icon icon="simple-icons:kofi" class="w-4 h-4" />
              <span class="text-xs">{$t("common.supportOnKofi")}</span>
            </button>
            <!-- Keyboard Shortcuts Info -->
            <div class="px-3 py-3 bg-white/5 rounded-lg mt-2">
              <p
                class="text-xs font-semibold text-white/60 mb-2 flex items-center gap-2"
              >
                <Icon icon="mdi:keyboard" class="w-4 h-4" />
                {$t("settings.shortcuts.title")}
              </p>
              <div class="space-y-1">
                {#each shortcuts.slice(0, 4) as shortcut}
                  <div class="flex justify-between text-xs">
                    <span class="text-white/40">{shortcut.action}</span>
                    <span class="text-white/60 font-mono"
                      >{shortcut.key.split(" / ")[0]}</span
                    >
                  </div>
                {/each}
              </div>
            </div>
          </aside>

          <!-- Main Content -->
          <div class="flex-1 flex flex-col overflow-hidden spotify-main">
            <!-- Content Area with transitions -->
            <div
              class="flex-1 overflow-hidden px-6 py-4 {$isDraggable
                ? 'drag-handle'
                : ''} no-drag"
            >
              {#key activeView}
                <div
                  class="h-full"
                  in:fly={{ y: 10, duration: 200, delay: 50 }}
                  out:fade={{ duration: 100 }}
                >
                  {#if activeView === "library"}
                    <MidiFileList />
                  {:else if activeView === "queue"}
                    <PlaylistManager />
                  {:else if activeView === "favorites"}
                    <FavoritesView />
                  {:else if activeView === "playlists"}
                    <SavedPlaylistsView />
                  {:else if activeView === "band"}
                    <BandMode on:selectsong={handleBandSelectSong} />
                  {:else if activeView === "share"}
                    <LibraryShare />
                  {:else if activeView === "stats"}
                    <StatsView />
                  {:else if activeView === "settings"}
                    <SettingsView />
                  {:else if activeView === "live"}
                    <LivePlayView />
                  {/if}
                </div>
              {/key}
            </div>
          </div>
        </div>

        <!-- Visualizer Bar (commented out)
        {#if showVisualizer}
          <div class="h-24 bg-[#0a0a0a] border-t border-white/5 no-drag">
            <Visualizer />
          </div>
        {/if}
        -->

        <!-- Bottom Player Bar -->
        <div
          class="spotify-player px-4 py-3 flex items-center justify-between gap-4 no-drag"
        >
          <!-- Now Playing -->
          <div class="flex items-center gap-4 w-64">
            <div
              class="relative w-12 h-12 rounded bg-white/5 flex items-center justify-center flex-shrink-0"
              title={gameFound ? $t("common.gameFound") : $t("common.gameNotFound")}
            >
              {#if $currentFile}
                <Icon icon="mdi:music-note" class="w-6 h-6 text-[#1db954]" />
              {:else}
                <Icon icon="mdi:music-note-off" class="w-6 h-6 text-white/30" />
              {/if}
              <!-- Game Status Dot -->
              <div class="absolute -top-1 -right-1 w-3 h-3 rounded-full border-2 border-[#121212] {gameFound ? 'bg-[#1db954]' : 'bg-red-500'} {gameFound && $isPlaying ? 'animate-pulse' : ''}"></div>
              <!-- Library Mode Indicator Dot -->
              {#if $libraryPlayMode}
                <div class="absolute -bottom-1 -right-1 w-3 h-3 rounded-full border-2 border-[#121212] bg-purple-500" title={$t("player.libraryPlayMode")}></div>
              {/if}
            </div>
            <div class="min-w-0 flex-1">
              <p class="text-sm font-semibold truncate text-white/90">
                {filename($currentFile, $t("player.noTrackSelected"))}
              </p>
              <p class="text-xs text-white/50 truncate">
                {#if $libraryPlayMode}
                  <span class="text-purple-400 flex items-center gap-1">
                    <Icon icon={$libraryPlayShuffle ? "mdi:shuffle" : "mdi:library-music"} class="w-3 h-3 inline" />
                    {$libraryPlayShuffle ? $t("player.shuffle") : $t("nav.library")} • {($libraryPlayIndex + 1).toLocaleString()} / {$midiFiles.length.toLocaleString()}
                  </span>
                {:else if $playlist.length > 0}
                  {$playlist.length} {$t("library.tracks")}
                {:else}
                  {$t("player.noTrackSelected")}
                {/if}
              </p>
            </div>
            <div class="flex items-center gap-1 flex-shrink-0">
              {#if $libraryPlayMode}
                <button
                  class="p-1.5 rounded-full text-purple-400 hover:text-purple-300 hover:bg-purple-500/10 transition-all"
                  onclick={exitLibraryPlayMode}
                  title="Exit library play mode"
                >
                  <Icon icon="mdi:close-circle" class="w-5 h-5" />
                </button>
              {/if}
              {#if $currentFile}
                <button
                  class="p-1.5 rounded-full transition-all {currentFileIsFavorite ? 'text-[#1db954]' : 'text-white/30 hover:text-white'}"
                  onclick={toggleCurrentFavorite}
                  title={currentFileIsFavorite ? "Remove from favorites" : "Add to favorites"}
                >
                  <Icon icon={currentFileIsFavorite ? "mdi:heart" : "mdi:heart-outline"} class="w-5 h-5" />
                </button>
              {/if}
            </div>
          </div>

          <!-- Player Controls Center -->
          <div class="flex-1 max-w-xl">
            <PlaybackControls {keybindings} />
            <Timeline />
            <!-- Settings Row -->
            <div class="flex items-center justify-center gap-3 mt-2">
              <!-- Speed -->
              <div class="relative">
                <button
                  class="flex items-center gap-1 px-2 py-1 rounded-md transition-colors text-xs font-medium {$speed !== 1.0 ? 'bg-[#1db954]/20 text-[#1db954]' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                  onclick={() => showSpeedMenu = !showSpeedMenu}
                  title="Playback speed"
                >
                  <Icon icon="mdi:speedometer" class="w-3.5 h-3.5" />
                  <span>{$speed}x</span>
                </button>
                {#if showSpeedMenu}
                  <button class="fixed inset-0 z-40" onclick={() => showSpeedMenu = false}></button>
                  <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 min-w-[90px]" in:fly={{ y: 5, duration: 150 }} out:fade={{ duration: 100 }}>
                    {#each speedOptions as option}
                      <button class="w-full px-3 py-1.5 text-xs text-left transition-colors {$speed === option.value ? 'bg-[#1db954]/20 text-[#1db954]' : 'text-white/70 hover:bg-white/5'}" onclick={() => selectSpeed(option.value)}>{option.label}</button>
                    {/each}
                  </div>
                {/if}
              </div>

              <!-- Key Mode -->
              <button
                class="flex items-center gap-1 px-2 py-1 rounded-md transition-colors text-xs font-medium {$keyMode === 'Keys36' ? 'bg-[#1db954]/20 text-[#1db954]' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                onclick={() => setKeyMode($keyMode === 'Keys21' ? 'Keys36' : 'Keys21')}
                title={$keyMode === 'Keys21' ? '21 keys (natural notes)' : '36 keys (with sharps/flats)'}
              >
                <Icon icon="mdi:piano" class="w-3.5 h-3.5" />
                <span>{$keyMode === 'Keys21' ? '21' : '36'}</span>
              </button>

              <!-- Octave -->
              <div class="flex items-center gap-1 px-1 py-0.5 rounded-md bg-white/5">
                <button class="w-5 h-5 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/10 transition-colors disabled:opacity-30" onclick={() => setOctaveShift($octaveShift - 1)} disabled={$octaveShift <= -2} title="Lower octave">
                  <Icon icon="mdi:minus" class="w-3 h-3" />
                </button>
                <span class="text-xs font-mono w-6 text-center {$octaveShift === 0 ? 'text-white/50' : $octaveShift > 0 ? 'text-[#1db954]' : 'text-orange-400'}" title="Octave shift">{$octaveShift > 0 ? '+' : ''}{$octaveShift}</span>
                <button class="w-5 h-5 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/10 transition-colors disabled:opacity-30" onclick={() => setOctaveShift($octaveShift + 1)} disabled={$octaveShift >= 2} title="Higher octave">
                  <Icon icon="mdi:plus" class="w-3 h-3" />
                </button>
              </div>

              <!-- Mode -->
              <div class="relative">
                <button
                  class="flex items-center gap-1 px-2 py-1 rounded-md transition-colors text-xs font-medium {$noteMode === 'Python' ? 'bg-pink-500/20 text-pink-400' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                  onclick={() => showModeMenu = !showModeMenu}
                  title="Note calculation mode"
                >
                  <Icon icon={noteModeOptions.find(m => m.id === $noteMode)?.icon || "mdi:music-note"} class="w-3.5 h-3.5" />
                  <span>{noteModeOptions.find(m => m.id === $noteMode)?.short || "CLS"}</span>
                </button>
                {#if showModeMenu}
                  <button class="fixed inset-0 z-40" onclick={() => showModeMenu = false}></button>
                  <div
                    class="absolute bottom-full right-0 mb-2 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 min-w-[200px]"
                    in:fly={{ y: 10, duration: 150 }}
                    out:fade={{ duration: 100 }}
                  >
                    <div class="py-1">
                      {#each noteModeOptions as mode}
                        <button
                          class="w-full flex items-center gap-2 px-3 py-2 text-left transition-colors {$noteMode === mode.id ? 'bg-[#1db954]/20' : 'hover:bg-white/5'}"
                          onclick={() => selectNoteMode(mode.id)}
                        >
                          <Icon icon={mode.icon} class="w-4 h-4 flex-shrink-0 {$noteMode === mode.id ? 'text-[#1db954]' : 'text-white/50'}" />
                          <div class="flex-1 min-w-0">
                            <div class="flex items-center gap-1.5 text-sm font-medium {$noteMode === mode.id ? 'text-[#1db954]' : 'text-white/90'}">
                              {mode.title || mode.id}
                              {#if mode.isRmd}
                                <span class="px-1.5 text-[10px] font-semibold bg-[#1db954]/20 text-[#1db954] rounded-full leading-4">RMD</span>
                              {/if}
                            </div>
                            <div class="text-xs {$noteMode === mode.id ? 'text-[#1db954]/70' : 'text-white/40'}">{mode.desc}</div>
                          </div>
                          {#if $noteMode === mode.id}
                            <Icon icon="mdi:check" class="w-4 h-4 text-[#1db954] flex-shrink-0" />
                          {/if}
                        </button>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>

              <!-- Track Selector (hidden in band mode) -->
              {#if $availableTracks.length > 1 && $bandStatus !== 'connected'}
                <div class="relative">
                  <button
                    class="flex items-center gap-1 px-2 py-1 rounded-md transition-colors text-xs font-medium {$selectedTrackId !== null ? 'bg-purple-500/20 text-purple-400' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                    onclick={() => showTrackMenu = !showTrackMenu}
                    title={$t("trackSelector.selectTrack")}
                  >
                    <Icon icon="mdi:playlist-music" class="w-3.5 h-3.5" />
                    <span class="max-w-[60px] truncate">
                      {#if $selectedTrackId === null}
                        {$t("trackSelector.all")}
                      {:else}
                        {$availableTracks.find(t => t.id === $selectedTrackId)?.name || $t("trackSelector.trackNum", { values: { num: $selectedTrackId + 1 } })}
                      {/if}
                    </span>
                  </button>
                  {#if showTrackMenu}
                    <button class="fixed inset-0 z-40" onclick={() => showTrackMenu = false}></button>
                    <div
                      class="absolute bottom-full right-0 mb-2 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 min-w-[180px] max-w-[250px] max-h-[300px] overflow-y-auto scrollbar-thin"
                      in:fly={{ y: 10, duration: 150 }}
                      out:fade={{ duration: 100 }}
                    >
                      <div class="py-1">
                        <!-- All tracks option -->
                        <button
                          class="w-full flex items-center gap-2 px-3 py-2 text-left transition-colors {$selectedTrackId === null ? 'bg-purple-500/20' : 'hover:bg-white/5'}"
                          onclick={() => selectTrack(null)}
                        >
                          <Icon icon="mdi:playlist-play" class="w-4 h-4 flex-shrink-0 {$selectedTrackId === null ? 'text-purple-400' : 'text-white/50'}" />
                          <div class="flex-1 min-w-0">
                            <div class="text-sm font-medium {$selectedTrackId === null ? 'text-purple-400' : 'text-white/90'}">{$t("trackSelector.allTracks")}</div>
                            <div class="text-xs {$selectedTrackId === null ? 'text-purple-400/70' : 'text-white/40'}">{$t("trackSelector.playEverything")}</div>
                          </div>
                          {#if $selectedTrackId === null}
                            <Icon icon="mdi:check" class="w-4 h-4 text-purple-400 flex-shrink-0" />
                          {/if}
                        </button>
                        <!-- Divider -->
                        <div class="h-px bg-white/10 my-1"></div>
                        <!-- Individual tracks -->
                        {#each $availableTracks as track}
                          <button
                            class="w-full flex items-center gap-2 px-3 py-2 text-left transition-colors {$selectedTrackId === track.id ? 'bg-purple-500/20' : 'hover:bg-white/5'}"
                            onclick={() => selectTrack(track.id)}
                          >
                            <Icon icon="mdi:music-note" class="w-4 h-4 flex-shrink-0 {$selectedTrackId === track.id ? 'text-purple-400' : 'text-white/50'}" />
                            <div class="flex-1 min-w-0">
                              <div class="text-sm font-medium truncate {$selectedTrackId === track.id ? 'text-purple-400' : 'text-white/90'}">{track.name}</div>
                              <div class="text-xs {$selectedTrackId === track.id ? 'text-purple-400/70' : 'text-white/40'}">{$t("trackSelector.notes", { values: { count: track.note_count } })}</div>
                            </div>
                            {#if $selectedTrackId === track.id}
                              <Icon icon="mdi:check" class="w-4 h-4 text-purple-400 flex-shrink-0" />
                            {/if}
                          </button>
                        {/each}
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}

              <!-- Visualizer Toggle (commented out)
              <button
                class="flex items-center gap-1 px-2 py-1 rounded-md transition-colors text-xs font-medium {showVisualizer ? 'bg-[#1db954]/20 text-[#1db954]' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                onclick={() => showVisualizer = !showVisualizer}
                title="Toggle visualizer"
              >
                <Icon icon="mdi:chart-bar" class="w-3.5 h-3.5" />
              </button>
              -->

              <!-- Update Available -->
              {#if updateAvailable}
                <button
                  class="flex items-center gap-1 px-2 py-1 rounded-md bg-[#1db954]/20 text-white hover:bg-[#1db954]/30 transition-colors text-xs font-medium"
                  onclick={() => showUpdateModal = true}
                  title="New version available"
                >
                  <Icon icon="mdi:download" class="w-3.5 h-3.5" />
                  <span>v{updateAvailable.version}</span>
                </button>
              {/if}

              <!-- Language Switcher -->
              <div class="relative">
                <button
                  class="flex items-center gap-1 px-2 py-1 rounded-md transition-colors text-xs font-medium text-white/50 hover:text-white hover:bg-white/5"
                  onclick={() => showLanguageMenu = !showLanguageMenu}
                  title="Language"
                >
                  <Icon icon={$currentLanguageInfo?.flag || 'circle-flags:us'} class="w-4 h-4" />
                </button>
                {#if showLanguageMenu}
                  <button class="fixed inset-0 z-40" onclick={() => showLanguageMenu = false}></button>
                  <div
                    class="absolute bottom-full right-0 mb-2 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 min-w-[140px]"
                    in:fly={{ y: 10, duration: 150 }}
                    out:fade={{ duration: 100 }}
                  >
                    {#each languages as lang}
                      <button
                        class="w-full flex items-center gap-2 px-3 py-2 text-left text-sm transition-colors {$currentLanguage === lang.code ? 'bg-[#1db954]/20 text-[#1db954]' : 'text-white/80 hover:bg-white/5'}"
                        onclick={() => { setLanguage(lang.code); showLanguageMenu = false; }}
                      >
                        <Icon icon={lang.flag} class="w-4 h-4" />
                        <span>{lang.name}</span>
                        {#if $currentLanguage === lang.code}
                          <Icon icon="mdi:check" class="w-4 h-4 ml-auto" />
                        {/if}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>

          <!-- Right Controls -->
          <div class="flex items-center gap-2 w-48 justify-end">
            <button
              class="spotify-icon-button"
              onclick={() => (activeView = "queue")}
              title="View queue"
            >
              <Icon icon="mdi:playlist-play" class="w-4 h-4" />
            </button>
          </div>
        </div>
      {:else}
        <!-- Minimized view -->
        <div class="spotify-player p-4">
          <PlaybackControls compact={true} {keybindings} />
          <Timeline compact={true} />
        </div>
      {/if}
    </div>
  </main>
{/if}

<!-- Update Modal -->
{#if showUpdateModal && updateAvailable}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <!-- Backdrop -->
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { if (updateStatus === 'idle' || updateStatus === 'downloaded' || updateStatus === 'error') showUpdateModal = false; }}
    ></button>

    <!-- Modal -->
    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[400px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-white/10">
        <h3 class="text-lg font-bold">{$t("modals.update.title")}</h3>
        {#if updateStatus === 'idle' || updateStatus === 'downloaded' || updateStatus === 'error'}
          <button
            class="p-1 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors"
            onclick={() => showUpdateModal = false}
          >
            <Icon icon="mdi:close" class="w-5 h-5" />
          </button>
        {/if}
      </div>

      <!-- Content -->
      <div class="p-4 space-y-4">
        <div class="flex items-center gap-4">
          <div class="w-16 h-16 rounded-xl bg-[#1db954]/10 flex items-center justify-center">
            {#if updateStatus === 'downloading'}
              <Icon icon="mdi:loading" class="w-10 h-10 text-[#1db954] animate-spin" />
            {:else if updateStatus === 'downloaded'}
              <Icon icon="mdi:check-circle" class="w-10 h-10 text-[#1db954]" />
            {:else if updateStatus === 'installing'}
              <Icon icon="mdi:cog" class="w-10 h-10 text-[#1db954] animate-spin" />
            {:else if updateStatus === 'error'}
              <Icon icon="mdi:alert-circle" class="w-10 h-10 text-red-400" />
            {:else}
              <Icon icon="mdi:download-circle" class="w-10 h-10 text-[#1db954]" />
            {/if}
          </div>
          <div>
            <p class="text-2xl font-bold text-[#1db954]">v{updateAvailable.version}</p>
            <p class="text-sm text-white/50">{$t("modals.update.current")} v{APP_VERSION}{APP_FLAVOR ? `(${APP_FLAVOR})` : ''}</p>
          </div>
        </div>

        {#if updateStatus === 'error'}
          <div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20">
            <p class="text-sm text-red-400">{updateError}</p>
          </div>
        {:else if updateStatus === 'downloading'}
          <p class="text-sm text-white/70">{$t("modals.update.downloading")}</p>
        {:else if updateStatus === 'downloaded'}
          <p class="text-sm text-white/70">{$t("modals.update.downloadComplete")}</p>
        {:else if updateStatus === 'installing'}
          <p class="text-sm text-white/70">{$t("modals.update.installing")}</p>
        {:else}
          <p class="text-sm text-white/70">{$t("modals.update.newVersionAvailable")}</p>
        {/if}

        <!-- Action Buttons -->
        <div class="flex gap-2 pt-2">
          {#if updateStatus === 'idle'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => invoke('open_url', { url: updateAvailable.release_url })}
            >
              {$t("modals.update.manual")}
            </button>
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors flex items-center justify-center gap-2"
              onclick={downloadUpdate}
            >
              <Icon icon="mdi:download" class="w-4 h-4" />
              {$t("modals.update.autoUpdate")}
            </button>
          {:else if updateStatus === 'downloading'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 text-white/50 font-medium text-sm cursor-not-allowed flex items-center justify-center gap-2"
              disabled
            >
              <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              {$t("common.loading")}
            </button>
          {:else if updateStatus === 'downloaded'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => showUpdateModal = false}
            >
              {$t("modals.update.later")}
            </button>
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors flex items-center justify-center gap-2"
              onclick={installUpdate}
            >
              <Icon icon="mdi:restart" class="w-4 h-4" />
              {$t("modals.update.installRestart")}
            </button>
          {:else if updateStatus === 'installing'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 text-white/50 font-medium text-sm cursor-not-allowed flex items-center justify-center gap-2"
              disabled
            >
              <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              {$t("common.loading")}
            </button>
          {:else if updateStatus === 'error'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => { updateStatus = 'idle'; updateError = ''; }}
            >
              {$t("modals.update.tryAgain")}
            </button>
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => invoke('open_url', { url: updateAvailable.release_url })}
            >
              {$t("modals.update.manualDownload")}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Large Library Warning Modal -->
{#if showLargeLibraryModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <!-- Backdrop -->
    <div class="absolute inset-0 bg-black/60"></div>

    <!-- Modal -->
    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[450px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-white/10">
        <h3 class="text-lg font-bold flex items-center gap-2">
          <Icon icon="mdi:alert-circle" class="w-5 h-5 text-yellow-400" />
          {$t("modals.largeLibrary.title")}
        </h3>
      </div>

      <!-- Content -->
      <div class="p-4 space-y-4">
        <div class="flex items-center gap-4">
          <div class="w-16 h-16 rounded-xl bg-yellow-500/10 flex items-center justify-center">
            <Icon icon="mdi:folder-music" class="w-10 h-10 text-yellow-400" />
          </div>
          <div>
            <p class="text-2xl font-bold text-yellow-400">{largeLibraryCount.toLocaleString()}</p>
            <p class="text-sm text-white/50">{$t("modals.largeLibrary.filesFound")}</p>
          </div>
        </div>

        <p class="text-sm text-white/70">
          {$t("modals.largeLibrary.description")}
        </p>

        <div class="p-3 rounded-lg bg-white/5">
          <p class="text-xs text-white/50 mb-2">{$t("modals.largeLibrary.recommendation")}</p>
          <p class="text-sm text-white/70">
            {$t("modals.largeLibrary.recommendationText")}
          </p>
        </div>

        <p class="text-xs text-white/40 italic">
          {$t("modals.largeLibrary.onceMessage")}
        </p>

        <!-- Action Buttons -->
        <div class="flex gap-2 pt-2">
          <button
            class="flex-1 px-4 py-2.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors flex items-center justify-center gap-2"
            onclick={async () => {
              showLargeLibraryModal = false;
              await loadInitialBatch();
            }}
          >
            <Icon icon="mdi:lightning-bolt" class="w-4 h-4" />
            {$t("modals.largeLibrary.loadRecommended")}
          </button>
          <button
            class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white/70 font-medium text-sm transition-colors flex items-center justify-center gap-2"
            onclick={async () => {
              showLargeLibraryModal = false;
              await loadMidiFiles();
            }}
          >
            <Icon icon="mdi:download" class="w-4 h-4" />
            {$t("modals.largeLibrary.loadAll")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Close Confirmation Modal -->
{#if showCloseConfirmModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <button class="absolute inset-0 bg-black/60" onclick={() => showCloseConfirmModal = false}></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[350px] max-w-[90vw] overflow-hidden"
      transition:scale={{ duration: 150, start: 0.95 }}
    >
      <div class="p-5 space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-full bg-red-500/20 flex items-center justify-center">
            <Icon icon="mdi:power" class="w-5 h-5 text-red-400" />
          </div>
          <div>
            <h3 class="text-lg font-bold">{$t("modals.closeApp.title")}</h3>
            <p class="text-sm text-white/50">{$t("modals.closeApp.message")}</p>
          </div>
        </div>

        <div class="flex gap-2">
          <button
            class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
            onclick={() => showCloseConfirmModal = false}
          >
            {$t("modals.closeApp.cancel")}
          </button>
          <button
            class="flex-1 px-4 py-2.5 rounded-lg bg-red-500 hover:bg-red-600 text-white font-medium text-sm transition-colors flex items-center justify-center gap-2"
            onclick={closeApp}
          >
            <Icon icon="mdi:power" class="w-4 h-4" />
            {$t("modals.closeApp.exit")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Importing Files Overlay -->
{#if $isImportingFiles}
  <div
    class="fixed inset-0 z-[100] bg-black/70 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <div class="text-center">
      <Icon icon="mdi:loading" class="w-12 h-12 text-[#1db954] mx-auto mb-4 animate-spin" />
      <p class="text-lg font-semibold">{$t("modals.importing.title")}</p>
    </div>
  </div>
{/if}

<!-- Share Notification - Above Playback Bar -->
{#if $shareNotification && !$miniMode}
  <div
    class="fixed bottom-[118px] left-1/2 -translate-x-1/2 z-[60] w-full max-w-md px-4"
    transition:fly={{ y: 20, duration: 200 }}
  >
    <div class="p-2.5 rounded-lg bg-[#1db954]/15 border border-[#1db954]/30 backdrop-blur-md flex items-center gap-3">
      <Icon icon="mdi:upload" class="w-4 h-4 text-[#1db954] flex-shrink-0" />
      <p class="text-xs text-white/90 flex-1 truncate">
        <span class="text-[#1db954] font-medium">{$shareNotification.peerName}</span> {$t("share.downloaded")} <span class="text-white/70">{$shareNotification.songName}</span>
      </p>
      <button
        class="text-white/40 hover:text-white transition-colors flex-shrink-0"
        onclick={dismissShareNotification}
      >
        <Icon icon="mdi:close" class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>
{/if}

<style>
  :global(body) {
    background: transparent;
  }

  .nav-item {
    position: relative;
    overflow: hidden;
  }

  .nav-item::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.03));
    opacity: 0;
    transition: opacity 0.2s;
  }

  .nav-item:hover::before {
    opacity: 1;
  }
</style>
