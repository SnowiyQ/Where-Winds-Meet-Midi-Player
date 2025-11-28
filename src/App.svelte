<script>
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";

  // Current version
  const APP_VERSION = "1.0.6";

  // Game window detection
  let gameFound = false;
  let checkInterval;

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

  onDestroy(() => {
    if (checkInterval) clearInterval(checkInterval);
  });
  import { fade, fly } from "svelte/transition";
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
  import Visualizer from "./lib/components/Visualizer.svelte";

  import {
    loadMidiFiles,
    initializeListeners,
    isMinimized,
    isDraggable,
    miniMode,
    toggleMiniMode,
    currentFile,
    playlist,
    favorites,
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
  } from "./lib/stores/player.js";


  // Note mode options for quick selector
  const noteModeOptions = [
    { id: "Python", title: "YueLyn", short: "YL", icon: "mdi:heart", desc: "YueLyn's favorite play mode" },
    { id: "Closest", short: "CLS", icon: "mdi:target", desc: "Best fit for most songs" },
    { id: "Quantize", short: "QNT", icon: "mdi:grid", desc: "Snap to scale notes" },
    { id: "TransposeOnly", short: "TRP", icon: "mdi:arrow-up-down", desc: "Direct octave shift" },
    { id: "Pentatonic", short: "PEN", icon: "mdi:music", desc: "5-note scale mapping" },
    { id: "Chromatic", short: "CHR", icon: "mdi:piano", desc: "12 to 7 key mapping" },
    { id: "Raw", short: "RAW", icon: "mdi:code-braces", desc: "1:1 direct, no processing" },
  ];

  let showModeMenu = false;
  let showSpeedMenu = false;
  let showVisualizer = false;
  let showUpdateModal = false;

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

  let sidebarTab = "music"; // "music" or "app"

  $: musicNavItems = [
    { id: "library", icon: "mdi:library-music", label: "Library", badge: 0 },
    { id: "queue", icon: "mdi:playlist-play", label: "Queue", badge: queueCount },
    { id: "favorites", icon: "mdi:heart", label: "Favorites", badge: favoritesCount },
    { id: "playlists", icon: "mdi:folder-music", label: "Playlists", badge: playlistsCount },
  ];

  $: appNavItems = [
    { id: "stats", icon: "mdi:chart-bar", label: "Stats", badge: 0 },
    { id: "settings", icon: "mdi:cog", label: "Settings", badge: 0 },
  ];

  $: navItems = sidebarTab === "music" ? musicNavItems : appNavItems;

  const shortcuts = [
    { action: "Play / Pause", key: "F9" },
    { action: "Stop", key: "F12 / End" },
    { action: "Previous", key: "F10" },
    { action: "Next", key: "F11" },
    { action: "Mode", key: "[ / ]" },
  ];

  onMount(async () => {
    await loadMidiFiles();
    initializeListeners();
    checkForUpdates(); // Check for updates on startup

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
    };
  });

  const filename = (path) => {
    if (!path) return "No track selected";
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1] || path;
  };

  // Handle keyboard shortcuts when app is focused
  async function handleKeydown(event) {
    // Skip if user is typing in an input
    if (event.target.tagName === 'INPUT' || event.target.tagName === 'TEXTAREA') {
      return;
    }

    switch (event.key) {
      case 'F9':
        event.preventDefault();
        await pauseResume();
        break;
      case 'F10':
        event.preventDefault();
        await playPrevious();
        break;
      case 'F11':
        event.preventDefault();
        await playNext();
        break;
      case 'F12':
      case 'End':
        event.preventDefault();
        await stopPlayback();
        break;
      case '[':
        event.preventDefault();
        prevNoteMode();
        break;
      case ']':
        event.preventDefault();
        nextNoteMode();
        break;
      case 'Insert':
        event.preventDefault();
        toggleMiniMode();
        break;
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
  <div class="flex flex-col items-center">
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
      class="h-screen w-full flex flex-col overflow-hidden rounded-md {$isDraggable
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
            <!-- Drag Handle with Mini Mode Toggle -->
            <div
              class="flex items-center justify-between py-2 -mx-4 -mt-4 mb-2 px-2"
            >
              <div class="w-8"></div>
              <div
                class="drag-handle flex-1 flex items-center justify-center cursor-move hover:bg-white/5 transition-colors group py-1 rounded"
                title="Drag to move window"
              >
                <Icon
                  icon="mdi:drag-horizontal"
                  class="w-6 h-6 text-white/20 group-hover:text-white/40 transition-colors"
                />
              </div>
              <button
                class="w-8 h-8 flex items-center justify-center rounded-lg text-white/40 hover:text-white hover:bg-white/10 transition-all"
                onclick={toggleMiniMode}
                title="Minimize to floating icon"
              >
                <Icon icon="mdi:minus" class="w-5 h-5" />
              </button>
            </div>

            <!-- Logo / Title -->
            <!-- <div class="px-3 py-2 mb-2 -mt-2"> -->
            <!-- <h1 class="text-lg font-bold text-white/90">WWM Overlay</h1> -->
            <!-- <p class="text-xs text-white/40">By YueLyn</p> -->
            <!-- </div> -->

            <!-- Sidebar Tabs -->
            <div class="flex gap-1 mb-2">
              <button
                class="flex-1 py-1.5 px-2 rounded-lg text-xs font-medium transition-all {sidebarTab === 'music' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                onclick={() => sidebarTab = 'music'}
              >
                <Icon icon="mdi:music" class="w-4 h-4 inline mr-1" />
                Music
              </button>
              <button
                class="flex-1 py-1.5 px-2 rounded-lg text-xs font-medium transition-all {sidebarTab === 'app' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white hover:bg-white/5'}"
                onclick={() => sidebarTab = 'app'}
              >
                <Icon icon="mdi:cog" class="w-4 h-4 inline mr-1" />
                App
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
                  {#if item.badge > 0}
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

            <p class="text-xs text-white/40 px-3">By YueLyn · v{APP_VERSION}</p>

            <!-- Ko-fi Support -->
            <button
              onclick={() => invoke('open_url', { url: 'https://ko-fi.com/snowiy' })}
              class="flex items-center gap-2 px-3 py-2 rounded-lg text-white/60 hover:text-[#ff5e5b] hover:bg-[#ff5e5b]/10 transition-all w-full mt-1"
              title="Support me on Ko-fi"
            >
              <Icon icon="simple-icons:kofi" class="w-4 h-4" />
              <span class="text-xs">Support on Ko-fi</span>
            </button>
            <!-- Keyboard Shortcuts Info -->
            <div class="px-3 py-3 bg-white/5 rounded-lg mt-2">
              <p
                class="text-xs font-semibold text-white/60 mb-2 flex items-center gap-2"
              >
                <Icon icon="mdi:keyboard" class="w-4 h-4" />
                Shortcuts
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
                  {:else if activeView === "stats"}
                    <StatsView />
                  {:else if activeView === "settings"}
                    <SettingsView />
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
              title={gameFound ? "Game window found" : "Game window not found"}
            >
              {#if $currentFile}
                <Icon icon="mdi:music-note" class="w-6 h-6 text-[#1db954]" />
              {:else}
                <Icon icon="mdi:music-note-off" class="w-6 h-6 text-white/30" />
              {/if}
              <!-- Game Status Dot -->
              <div class="absolute -top-1 -right-1 w-3 h-3 rounded-full border-2 border-[#121212] {gameFound ? 'bg-[#1db954]' : 'bg-red-500'} {gameFound && $isPlaying ? 'animate-pulse' : ''}"></div>
            </div>
            <div class="min-w-0">
              <p class="text-sm font-semibold truncate text-white/90">
                {filename($currentFile)}
              </p>
              <p class="text-xs text-white/50 truncate">
                {#if $playlist.length > 0}
                  {$playlist.length} tracks in queue
                {:else}
                  No tracks in queue
                {/if}
              </p>
            </div>
          </div>

          <!-- Player Controls Center -->
          <div class="flex-1 max-w-xl">
            <PlaybackControls />
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
                            <div class="text-sm font-medium {$noteMode === mode.id ? 'text-[#1db954]' : 'text-white/90'}">{mode.title || mode.id}</div>
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
            </div>
          </div>

          <!-- Right Controls -->
          <div class="flex items-center gap-2 w-48 justify-end">
            <button
              class="spotify-icon-button transition-all duration-200 {$loopMode
                ? 'text-[#1db954] bg-[#1db954]/10'
                : 'hover:text-white'}"
              onclick={toggleLoop}
              title={$loopMode ? "Loop enabled" : "Enable loop"}
            >
              <Icon icon="mdi:repeat" class="w-4 h-4" />
            </button>
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
          <PlaybackControls compact={true} />
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
        <h3 class="text-lg font-bold">Update Available</h3>
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
            <p class="text-sm text-white/50">Current: v{APP_VERSION}</p>
          </div>
        </div>

        {#if updateStatus === 'error'}
          <div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20">
            <p class="text-sm text-red-400">{updateError}</p>
          </div>
        {:else if updateStatus === 'downloading'}
          <p class="text-sm text-white/70">Downloading update... Please wait.</p>
        {:else if updateStatus === 'downloaded'}
          <p class="text-sm text-white/70">Download complete! Click "Install & Restart" to apply the update, or restart later manually.</p>
        {:else if updateStatus === 'installing'}
          <p class="text-sm text-white/70">Installing update... The app will restart automatically.</p>
        {:else}
          <p class="text-sm text-white/70">A new version is available! Download and install automatically, or download manually from GitHub.</p>
        {/if}

        <!-- Action Buttons -->
        <div class="flex gap-2 pt-2">
          {#if updateStatus === 'idle'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => invoke('open_url', { url: updateAvailable.release_url })}
            >
              Manual
            </button>
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors flex items-center justify-center gap-2"
              onclick={downloadUpdate}
            >
              <Icon icon="mdi:download" class="w-4 h-4" />
              Auto Update
            </button>
          {:else if updateStatus === 'downloading'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 text-white/50 font-medium text-sm cursor-not-allowed flex items-center justify-center gap-2"
              disabled
            >
              <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              Downloading...
            </button>
          {:else if updateStatus === 'downloaded'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => showUpdateModal = false}
            >
              Later
            </button>
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors flex items-center justify-center gap-2"
              onclick={installUpdate}
            >
              <Icon icon="mdi:restart" class="w-4 h-4" />
              Install & Restart
            </button>
          {:else if updateStatus === 'installing'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 text-white/50 font-medium text-sm cursor-not-allowed flex items-center justify-center gap-2"
              disabled
            >
              <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              Installing...
            </button>
          {:else if updateStatus === 'error'}
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => { updateStatus = 'idle'; updateError = ''; }}
            >
              Try Again
            </button>
            <button
              class="flex-1 px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
              onclick={() => invoke('open_url', { url: updateAvailable.release_url })}
            >
              Manual Download
            </button>
          {/if}
        </div>
      </div>
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
