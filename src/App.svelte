<script>
  import { onMount } from "svelte";
  import { register, unregisterAll } from "@tauri-apps/plugin-global-shortcut";
  import Icon from "@iconify/svelte";

  import Header from "./lib/components/Header.svelte";
  import MidiFileList from "./lib/components/MidiFileList.svelte";
  import PlaybackControls from "./lib/components/PlaybackControls.svelte";
  import Timeline from "./lib/components/Timeline.svelte";
  import PlaylistManager from "./lib/components/PlaylistManager.svelte";

  import {
    loadMidiFiles,
    initializeListeners,
    isMinimized,
    isDraggable,
    currentFile,
    playlist,
    smartPause,
    loopMode,
    pauseResume,
    stopPlayback,
    playNext,
    playPrevious,
    toggleLoop,
    toggleDraggable,
  } from "./lib/stores/player.js";

  let showPlaylist = false;
  const shortcuts = [
    { action: "Play / Pause", key: "Scroll Lock" },
    { action: "Stop", key: "End" },
    { action: "Previous", key: "Ctrl + P" },
    { action: "Next", key: "Ctrl + N" },
    { action: "Toggle Loop", key: "Ctrl + L" },
  ];

  onMount(async () => {
    await loadMidiFiles();
    initializeListeners();
    await registerGlobalShortcuts();

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      unregisterAll();
    };
  });

  async function registerGlobalShortcuts() {
    try {
      await unregisterAll();
      const combos = [
        { shortcut: "ScrollLock", action: pauseResume },
        { shortcut: "End", action: stopPlayback },
        { shortcut: "Control+P", action: playPrevious },
        { shortcut: "Control+N", action: playNext },
        { shortcut: "Control+L", action: toggleLoop },
      ];

      for (const combo of combos) {
        await register(combo.shortcut, async (event) => {
          if (event.state === "Pressed") {
            await combo.action();
          }
        });
      }
    } catch (error) {
      console.error("Failed to register global shortcuts:", error);
    }
  }

  async function handleKeyDown(event) {
    const key = event.key.toLowerCase();
    if (event.key === "ScrollLock") {
      event.preventDefault();
      await pauseResume();
    } else if (event.key === "End") {
      event.preventDefault();
      await stopPlayback();
    } else if (event.ctrlKey && key === "n") {
      event.preventDefault();
      await playNext();
    } else if (event.ctrlKey && key === "p") {
      event.preventDefault();
      await playPrevious();
    } else if (event.ctrlKey && key === "l") {
      event.preventDefault();
      await toggleLoop();
    }
  }

  const filename = (path) => {
    if (!path) return "No track selected";
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1] || path;
  };
</script>

<main
  class="h-screen w-full flex flex-col overflow-hidden {$isDraggable
    ? ''
    : 'pointer-events-none'}"
>
  {#if !$isMinimized}
    <!-- Spotify-style layout -->
    <div class="flex flex-1 min-h-0 overflow-hidden">
      <!-- Sidebar -->
      <aside
        class="spotify-main spotify-sidebar w-64 flex flex-col p-6 gap-6 no-drag"
      >
        <!-- Logo -->
        <!-- <div class="flex items-center gap-3">
          <img src="/src/icon.png" alt="WWM" class="w-10 h-10 rounded" />
        </div> -->

        <!-- Navigation -->
        <nav class="flex flex-col gap-2">
          <button
            class="spotify-list-item text-left"
            class:spotify-list-item--active={!showPlaylist}
            on:click={() => (showPlaylist = false)}
          >
            <div class="flex items-center gap-3">
              <Icon icon="mdi:library-music" class="w-5 h-5" />
              <span class="font-semibold">Library</span>
            </div>
          </button>
          <button
            class="spotify-list-item text-left"
            class:spotify-list-item--active={showPlaylist}
            on:click={() => (showPlaylist = true)}
          >
            <div class="flex items-center gap-3">
              <Icon icon="mdi:playlist-music" class="w-5 h-5" />
              <span class="font-semibold">Queue</span>
            </div>
          </button>
        </nav>

        <!-- Settings -->
        <!-- <div class="mt-auto space-y-4">
          <div class="flex items-center justify-between">
            <span class="text-sm text-white/70">Smart Pause</span>
            <button
              class="relative w-11 h-6 rounded-full transition-colors {$smartPause
                ? 'bg-[#1db954]'
                : 'bg-white/20'}"
              aria-label="Toggle smart pause"
              aria-pressed={$smartPause}
              on:click={() => smartPause.update((v) => !v)}
            >
              <div
                class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {$smartPause
                  ? 'translate-x-5'
                  : ''}"
              ></div>
            </button>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-sm text-white/70">Interactive</span>
            <button
              class="relative w-11 h-6 rounded-full transition-colors {$isDraggable
                ? 'bg-[#1db954]'
                : 'bg-white/20'}"
              aria-label="Toggle interaction mode"
              aria-pressed={$isDraggable}
              on:click={toggleDraggable}
            >
              <div
                class="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform {$isDraggable
                  ? 'translate-x-5'
                  : ''}"
              ></div>
            </button>
          </div>
        </div> -->
      </aside>

      <!-- Main Content -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Top Bar with window controls -->
        <div
          class="px-6 py-4 flex items-center justify-between right-0 z-[999] {$isDraggable
            ? 'drag-handle'
            : ''}"
        >
          <div class="flex items-center gap-2"></div>
          <div class="flex items-center gap-2 no-drag">
            <!-- <button
              class="spotify-icon-button"
              on:click={() => isMinimized.update((v) => !v)}
              title="Minimize"
            >
              <Icon icon="mdi:minus" class="w-4 h-4" />
            </button> -->
            <!-- <button class="spotify-icon-button drag-handle " title="Drag">
              <Icon icon="mdi:drag" class="w-4 h-4" />
            </button> -->
            <!-- <button
              class="spotify-icon-button hover:bg-red-500/20 hover:text-red-400"
              on:click={close}
              title="Close"
            >
              <Icon icon="mdi:close" class="w-4 h-4" />
            </button> -->
            <button
              class="spotify-icon-button no-drag"
              on:click={loadMidiFiles}
              title="Refresh"
            >
              <Icon icon="mdi:refresh" class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Content Area -->
        <div
          class="flex-1 overflow-hidden px-6 pb-6 no-drag -pt-6 -mt-10 drag-handle"
        >
          {#if showPlaylist}
            <PlaylistManager />
          {:else}
            <MidiFileList />
          {/if}
        </div>
      </div>
    </div>

    <!-- Bottom Player Bar -->
    <div
      class="spotify-player px-4 py-3 flex items-center justify-between gap-4 no-drag"
    >
      <!-- Now Playing -->
      <div class="flex items-center gap-4 w-72">
        <!-- <img
          src="/src/icon.png"
          alt="Album"
          class="w-14 h-14 rounded flex-shrink-0"
        /> -->
        <div class="min-w-0">
          <p class="text-sm font-semibold truncate">{filename($currentFile)}</p>
          <p class="text-xs text-white/60 truncate">
            {$playlist.length} tracks in queue
          </p>
        </div>
      </div>

      <!-- Player Controls Center -->
      <div class="flex-1 max-w-2xl">
        <PlaybackControls />
        <Timeline />
      </div>

      <!-- Right Controls -->
      <div class="flex items-center gap-2 w-72 justify-end">
        <button
          class="spotify-icon-button {$loopMode ? 'text-[#1db954]' : ''}"
          on:click={toggleLoop}
          title="Loop"
        >
          <Icon icon="mdi:repeat" class="w-4 h-4" />
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
</main>

<style>
  :global(body) {
    background: transparent;
  }
</style>
