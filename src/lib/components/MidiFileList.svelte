<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    midiFiles,
    currentFile,
    playMidi,
    playlist,
    isPlaying,
    isPaused,
    favorites,
    toggleFavorite,
    savedPlaylists,
    addToSavedPlaylist,
    importMidiFile,
    isLoadingMidi,
    midiLoadProgress,
    totalMidiCount,
    hasMoreFiles,
    loadMoreFiles,
    loadAllFiles,
    playAllLibrary,
    libraryPlayMode,
  } from "../stores/player.js";
  import { bandSongSelectMode, selectBandSong, cancelBandSongSelect } from "../stores/band.js";
  import SongContextMenu from "./SongContextMenu.svelte";

  let searchQuery = "";
  let showPlaylistMenu = null;
  let toast = null;
  let toastTimeout = null;
  let isDragOver = false;
  let isImporting = false;
  let unlistenDrop = null;
  let unlistenHover = null;
  let unlistenCancel = null;
  let sortBy = "name-asc"; // name-asc, name-desc, duration-asc, duration-desc
  let showSortMenu = false;

  // Import modal
  let showImportModal = false;
  let urlInput = "";
  let isDownloading = false;

  // Export library
  let isExporting = false;
  let exportProgress = { current: 0, total: 0 };

  // Context menu
  let contextMenu = null;

  // Scroll mask
  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

  // Virtual scrolling - only render visible items
  const ITEM_HEIGHT = 52; // Height of each item in pixels
  const BUFFER_ITEMS = 10; // Extra items to render above/below viewport
  let visibleStartIndex = 0;
  let visibleEndIndex = 100; // Initial render count
  let scrollTop = 0;

  // Autofocus action
  function autofocus(node) {
    node.focus();
  }

  function handleScroll(e) {
    const { scrollTop: st, scrollHeight, clientHeight } = e.target;
    scrollTop = st;
    showTopMask = st > 10;
    showBottomMask = st + clientHeight < scrollHeight - 10;

    // Update visible range for virtual scrolling
    const startIndex = Math.max(0, Math.floor(st / ITEM_HEIGHT) - BUFFER_ITEMS);
    const visibleCount = Math.ceil(clientHeight / ITEM_HEIGHT) + BUFFER_ITEMS * 2;
    visibleStartIndex = startIndex;
    visibleEndIndex = startIndex + visibleCount;
  }

  // Get visible slice of files for virtual scrolling
  $: visibleFiles = filteredFiles.slice(visibleStartIndex, Math.min(visibleEndIndex, filteredFiles.length));
  $: topPadding = visibleStartIndex * ITEM_HEIGHT;
  $: bottomPadding = Math.max(0, (filteredFiles.length - visibleEndIndex) * ITEM_HEIGHT);

  // Reset visible range when container height changes or search changes
  $: if (scrollContainer && filteredFiles.length > 0) {
    // Ensure we always have enough items visible
    const clientHeight = scrollContainer?.clientHeight || 500;
    const minVisible = Math.ceil(clientHeight / ITEM_HEIGHT) + BUFFER_ITEMS * 2;
    if (visibleEndIndex - visibleStartIndex < minVisible) {
      visibleEndIndex = Math.min(visibleStartIndex + minVisible, filteredFiles.length);
    }
  }

  onMount(async () => {
    // Check initial scroll state and initialize virtual scroll
    setTimeout(() => {
      if (scrollContainer) {
        const { scrollTop: st, scrollHeight, clientHeight } = scrollContainer;
        showBottomMask = scrollHeight > clientHeight;

        // Initialize visible range
        const startIndex = Math.max(0, Math.floor(st / ITEM_HEIGHT) - BUFFER_ITEMS);
        const visibleCount = Math.ceil(clientHeight / ITEM_HEIGHT) + BUFFER_ITEMS * 2;
        visibleStartIndex = startIndex;
        visibleEndIndex = Math.max(100, startIndex + visibleCount); // At least 100 items initially
      }
    }, 100);

    // Listen for Tauri drag-drop events
    unlistenDrop = await listen("tauri://drag-drop", async (event) => {
      isDragOver = false;
      const paths = event.payload.paths || [];
      const midFiles = paths.filter(p => p.toLowerCase().endsWith('.mid'));

      if (midFiles.length === 0) {
        showToast("Please drop .mid files only", "error");
        return;
      }

      await importFiles(midFiles);
    });

    unlistenHover = await listen("tauri://drag-enter", () => {
      isDragOver = true;
    });

    unlistenCancel = await listen("tauri://drag-leave", () => {
      isDragOver = false;
    });
  });

  onDestroy(() => {
    if (unlistenDrop) unlistenDrop();
    if (unlistenHover) unlistenHover();
    if (unlistenCancel) unlistenCancel();
  });

  async function importFiles(midFiles) {
    isImporting = true;
    let imported = 0;
    let failed = 0;

    for (const filePath of midFiles) {
      const result = await importMidiFile(filePath);
      if (result.success) {
        imported++;
      } else {
        failed++;
        console.error(`Failed to import:`, result.error);
      }
    }

    isImporting = false;

    if (imported > 0 && failed === 0) {
      showToast(`Imported ${imported} file${imported > 1 ? 's' : ''}`, "success");
    } else if (imported > 0 && failed > 0) {
      showToast(`Imported ${imported}, ${failed} failed`, "info");
    } else {
      showToast("Failed to import files", "error");
    }
  }

  async function openFileDialog() {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "MIDI Files", extensions: ["mid", "midi"] }],
      });

      if (selected && selected.length > 0) {
        showImportModal = false;
        await importFiles(selected);
      }
    } catch (error) {
      console.error("Failed to open file dialog:", error);
      showToast("Failed to open file dialog", "error");
    }
  }

  async function downloadFromUrl() {
    if (!urlInput.trim()) {
      showToast("Please enter a URL", "error");
      return;
    }

    const url = urlInput.trim();

    // Basic URL validation
    if (!url.startsWith('http://') && !url.startsWith('https://')) {
      showToast("Invalid URL format", "error");
      return;
    }

    isDownloading = true;
    try {
      const result = await invoke('download_midi_from_url', { url });
      showImportModal = false;
      urlInput = "";
      showToast(`Imported "${result.name}"`, "success");
      // Refresh file list
      const { loadMidiFiles } = await import('../stores/player.js');
      await loadMidiFiles();
    } catch (error) {
      console.error("Failed to download:", error);
      showToast(error.toString(), "error");
    } finally {
      isDownloading = false;
    }
  }

  async function exportLibrary() {
    if ($midiFiles.length === 0 || isExporting) return;

    try {
      isExporting = true;
      exportProgress = { current: 0, total: $totalMidiCount || $midiFiles.length };

      const exportPath = await save({
        title: "Export Library",
        defaultPath: "library.zip",
        filters: [{ name: "Zip Archive", extensions: ["zip"] }],
      });

      if (exportPath) {
        // Listen for progress events
        const unlisten = await listen("export-progress", (event) => {
          exportProgress = event.payload;
        });

        const count = await invoke("export_library", { exportPath });
        unlisten();
        showToast(`Exported ${count.toLocaleString()} songs`, "success");
      }
    } catch (error) {
      console.error("Failed to export library:", error);
      showToast(error.toString(), "error");
    } finally {
      isExporting = false;
      exportProgress = { current: 0, total: 0 };
    }
  }

  function showToast(message, type = "success") {
    if (toastTimeout) clearTimeout(toastTimeout);
    toast = { message, type };
    toastTimeout = setTimeout(() => {
      toast = null;
    }, 2000);
  }

  async function handlePlay(file) {
    // If in band song selection mode, select for band instead of playing
    if ($bandSongSelectMode) {
      await selectBandSong(file);
      return;
    }

    // Add to playlist if not already there
    playlist.update((list) => {
      if (!list.find((f) => f.path === file.path)) {
        return [...list, file];
      }
      return list;
    });
    await playMidi(file.path);
  }

  function addToQueue(file) {
    const added = !$playlist.find((f) => f.path === file.path);
    playlist.update((list) => {
      if (!list.find((f) => f.path === file.path)) {
        return [...list, file];
      }
      return list;
    });
    showToast(added ? "Added to queue" : "Already in queue", added ? "success" : "info");
  }

  function handleAddToPlaylist(playlistId, file) {
    const pl = $savedPlaylists.find(p => p.id === playlistId);
    const alreadyExists = pl?.tracks.some(t => t.path === file.path);
    addToSavedPlaylist(playlistId, file);
    showPlaylistMenu = null;
    showToast(
      alreadyExists ? `Already in "${pl?.name}"` : `Added to "${pl?.name}"`,
      alreadyExists ? "info" : "success"
    );
  }

  function handleToggleFavorite(file) {
    const wasFavorite = $favorites.some((f) => f.hash === file.hash);
    toggleFavorite(file);
    showToast(
      wasFavorite ? "Removed from favorites" : "Added to favorites",
      wasFavorite ? "info" : "success"
    );
  }

  // Reactive favorite lookup using a Set for O(1) performance (by hash for rename support)
  $: favoriteHashes = new Set($favorites.map(f => f.hash));

  // Check if a file is invalid/broken (no duration means parsing failed)
  function isInvalidFile(file) {
    return !file.duration || file.duration <= 0;
  }

  $: filteredFiles = $midiFiles
    .filter((file) => file.name.toLowerCase().includes(searchQuery.toLowerCase()))
    .sort((a, b) => {
      switch (sortBy) {
        case "name-asc":
          return a.name.localeCompare(b.name);
        case "name-desc":
          return b.name.localeCompare(a.name);
        case "duration-asc":
          return (a.duration || 0) - (b.duration || 0);
        case "duration-desc":
          return (b.duration || 0) - (a.duration || 0);
        case "bpm-asc":
          return (a.bpm || 120) - (b.bpm || 120);
        case "bpm-desc":
          return (b.bpm || 120) - (a.bpm || 120);
        case "density-asc":
          return (a.note_density || 0) - (b.note_density || 0);
        case "density-desc":
          return (b.note_density || 0) - (a.note_density || 0);
        default:
          return 0;
      }
    });

  // Context menu
  function handleContextMenu(e, file) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, file };
  }

  const sortOptions = [
    { id: "name-asc", label: "A-Z", icon: "mdi:sort-alphabetical-ascending" },
    { id: "name-desc", label: "Z-A", icon: "mdi:sort-alphabetical-descending" },
    { id: "duration-asc", label: "Shortest", icon: "mdi:sort-numeric-ascending" },
    { id: "duration-desc", label: "Longest", icon: "mdi:sort-numeric-descending" },
    { id: "bpm-asc", label: "Slow", icon: "mdi:speedometer-slow" },
    { id: "bpm-desc", label: "Fast", icon: "mdi:speedometer" },
    { id: "density-asc", label: "Easy", icon: "mdi:music-note" },
    { id: "density-desc", label: "Dense", icon: "mdi:music-note-plus" },
  ];

</script>

<div class="h-full flex flex-col relative">
  <!-- Drop Zone Overlay -->
  {#if isDragOver}
    <div
      class="absolute inset-0 z-40 bg-[#1db954]/10 border-2 border-dashed border-[#1db954] rounded-lg flex items-center justify-center"
      transition:fade={{ duration: 150 }}
    >
      <div class="text-center">
        <Icon icon="mdi:file-music" class="w-16 h-16 text-[#1db954] mx-auto mb-4" />
        <p class="text-lg font-semibold text-[#1db954]">Drop MIDI files here</p>
        <p class="text-sm text-white/60">Files will be added to your library</p>
      </div>
    </div>
  {/if}

  <!-- Importing Overlay -->
  {#if isImporting}
    <div
      class="absolute inset-0 z-40 bg-black/50 flex items-center justify-center"
      transition:fade={{ duration: 150 }}
    >
      <div class="text-center">
        <Icon icon="mdi:loading" class="w-12 h-12 text-[#1db954] mx-auto mb-4 animate-spin" />
        <p class="text-lg font-semibold">Importing files...</p>
      </div>
    </div>
  {/if}

  <!-- Band Selection Mode Banner -->
  {#if $bandSongSelectMode}
    <div
      class="mb-3 p-3 rounded-lg bg-[#1db954]/10 border border-[#1db954]/30 flex items-center gap-3"
      transition:fly={{ y: -10, duration: 200 }}
    >
      <Icon icon="mdi:account-group" class="w-5 h-5 text-[#1db954]" />
      <p class="text-sm flex-1">Select a song for Band Mode</p>
      <button
        class="text-xs text-white/50 hover:text-white transition-colors"
        onclick={cancelBandSongSelect}
      >
        Cancel
      </button>
    </div>
  {/if}

  <!-- Header -->
  <div class="mb-4">
    <div class="flex items-center justify-between mb-2">
      <h2 class="text-2xl font-bold">Your Library</h2>
      <div class="flex items-center gap-2">
        <!-- Play All / Shuffle All buttons -->
        {#if filteredFiles.length > 0 && !$isLoadingMidi}
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-[#1db954] hover:bg-[#1ed760] text-white text-sm font-medium transition-all"
            onclick={() => {
              playAllLibrary(filteredFiles, 0, false);
              showToast(`Playing ${filteredFiles.length.toLocaleString()} songs`, "success");
            }}
            title="Play all songs in library"
          >
            <Icon icon="mdi:play" class="w-4 h-4" />
            Play All
          </button>
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white/80 hover:text-white text-sm font-medium transition-all"
            onclick={() => {
              playAllLibrary(filteredFiles, 0, true);
              showToast(`Shuffling ${filteredFiles.length.toLocaleString()} songs`, "success");
            }}
            title="Shuffle and play all songs"
          >
            <Icon icon="mdi:shuffle" class="w-4 h-4" />
            Shuffle
          </button>
        {/if}
        {#if $midiFiles.length > 0 && !$isLoadingMidi}
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white/80 hover:text-white text-sm font-medium transition-all disabled:opacity-50"
            onclick={exportLibrary}
            disabled={isExporting}
            title="Export entire library as zip"
          >
            {#if isExporting}
              <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              {exportProgress.total > 0 ? `${Math.round(exportProgress.current / exportProgress.total * 100)}%` : 'Export'}
            {:else}
              <Icon icon="mdi:export" class="w-4 h-4" />
              Export
            {/if}
          </button>
        {/if}
        <button
          class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white/80 hover:text-white text-sm font-medium transition-all"
          onclick={() => showImportModal = true}
          title="Import MIDI files"
        >
          <Icon icon="mdi:plus" class="w-4 h-4" />
          Import
        </button>
      </div>
    </div>
    <p class="text-sm text-white/60 mb-4 flex items-center gap-2">
      {#if $isLoadingMidi}
        <span class="flex items-center gap-2">
          <Icon icon="mdi:loading" class="w-4 h-4 animate-spin text-[#1db954]" />
          {#if $midiLoadProgress.total > 0}
            Loading {$midiLoadProgress.loaded.toLocaleString()} / {$midiLoadProgress.total.toLocaleString()} songs...
          {:else}
            Scanning...
          {/if}
        </span>
      {:else}
        <span>{filteredFiles.length} of {$midiFiles.length} songs</span>
        <span class="text-white/30">•</span>
        <span class="text-xs text-white/30 flex items-center gap-1">
          <Icon icon="mdi:mouse" class="w-3 h-3" />
          Right-click to rename, delete, or open location
        </span>
      {/if}
    </p>

    <!-- Search Input with Sort -->
    <div class="flex gap-2">
      <div class="relative flex-1">
        <Icon
          icon="mdi:magnify"
          class="absolute left-3 top-1/2 -translate-y-1/2 text-white/40 w-5 h-5"
        />
        <input
          type="text"
          placeholder="Search songs..."
          bind:value={searchQuery}
          class="w-full bg-white/5 border border-white/10 rounded-full pl-10 pr-10 py-2.5 text-sm text-white placeholder-white/30 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent focus:bg-white/10 transition-all"
        />
        {#if searchQuery}
          <button
            onclick={() => (searchQuery = "")}
            class="absolute right-3 top-1/2 -translate-y-1/2 text-white/40 hover:text-white transition-colors"
            transition:fade={{ duration: 150 }}
          >
            <Icon icon="mdi:close-circle" class="w-5 h-5" />
          </button>
        {/if}
      </div>

      <!-- Sort Button -->
      <div class="relative">
        <button
          class="h-full px-3 bg-white/5 border border-white/10 rounded-full flex items-center gap-1.5 text-white/60 hover:text-white hover:bg-white/10 transition-all"
          onclick={(e) => {
            e.stopPropagation();
            showSortMenu = !showSortMenu;
          }}
          title="Sort by"
        >
          <Icon icon={sortOptions.find(o => o.id === sortBy)?.icon || "mdi:sort"} class="w-4 h-4" />
          <span class="text-xs font-medium">{sortOptions.find(o => o.id === sortBy)?.label}</span>
        </button>

        {#if showSortMenu}
          <div
            class="absolute right-0 top-full mt-1 w-36 bg-[#282828] rounded-lg shadow-xl border border-white/10 py-1 z-50"
            transition:fly={{ y: -5, duration: 150 }}
            onclick={(e) => e.stopPropagation()}
          >
            {#each sortOptions as option}
              <button
                class="w-full px-3 py-2 text-left text-sm flex items-center gap-2 transition-colors {sortBy === option.id ? 'text-[#1db954] bg-white/5' : 'text-white/80 hover:bg-white/10'}"
                onclick={(e) => {
                  e.stopPropagation();
                  sortBy = option.id;
                  showSortMenu = false;
                }}
              >
                <Icon icon={option.icon} class="w-4 h-4" />
                {option.label}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Song List (Scrollable) - show if we have files, even while loading more -->
  {#if $midiFiles.length > 0}
  <div
    bind:this={scrollContainer}
    onscroll={handleScroll}
    class="flex-1 overflow-y-auto pr-2 {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
  >
    <!-- Virtual scroll padding top -->
    <div style="height: {topPadding}px"></div>

    {#each visibleFiles as file, i (file.path)}
      {@const invalid = isInvalidFile(file)}
      {@const index = visibleStartIndex + i}
      <div
        class="group spotify-list-item flex items-center gap-4 py-2 transition-colors duration-150 {$currentFile ===
        file.path
          ? 'bg-white/10 ring-1 ring-white/5'
          : 'hover:bg-white/5'} {invalid ? 'opacity-60' : ''}"
        style="height: {ITEM_HEIGHT}px"
        title={invalid ? 'Invalid MIDI file - cannot parse' : ''}
        oncontextmenu={(e) => handleContextMenu(e, file)}
      >
        <!-- Number / Play Button / Playing Indicator -->
        <div class="w-8 flex items-center justify-center flex-shrink-0">
          {#if $currentFile === file.path && $isPlaying && !$isPaused}
            <!-- Playing indicator (animated bars) -->
            <div class="flex items-end gap-0.5 h-4">
              <div
                class="w-0.5 bg-[#1db954] rounded-full"
                style="height: 60%; animation: music-bar-1 0.6s ease-in-out infinite;"
              ></div>
              <div
                class="w-0.5 bg-[#1db954] rounded-full"
                style="height: 100%; animation: music-bar-2 0.8s ease-in-out infinite;"
              ></div>
              <div
                class="w-0.5 bg-[#1db954] rounded-full"
                style="height: 80%; animation: music-bar-3 0.7s ease-in-out infinite;"
              ></div>
            </div>
          {:else}
            <span
              class="text-sm text-white/40 {$currentFile === file.path
                ? 'text-[#1db954] font-semibold'
                : ''} group-hover:hidden">{index + 1}</span
            >
            {#if !invalid}
              <button
                class="hidden group-hover:flex items-center justify-center w-7 h-7 rounded-full bg-[#1db954] hover:scale-110 transition-transform shadow-lg"
                onclick={() => handlePlay(file)}
                title={$bandSongSelectMode ? "Select for Band" : "Play"}
              >
                <Icon icon={$bandSongSelectMode ? "mdi:check" : "mdi:play"} class="w-4 h-4 text-black" />
              </button>
            {/if}
          {/if}
        </div>

        <!-- Song Info -->
        <div
          class="flex-1 min-w-0 {invalid ? 'cursor-not-allowed' : ''}"
          role={invalid ? undefined : "button"}
          tabindex={invalid ? undefined : 0}
          onclick={() => !invalid && handlePlay(file)}
          onkeydown={(event) => {
            if (!invalid && (event.key === "Enter" || event.key === " ")) {
              event.preventDefault();
              handlePlay(file);
            }
          }}
        >
          <p
            class="text-sm font-medium truncate transition-colors {invalid
              ? 'text-red-400'
              : $currentFile === file.path
                ? 'text-[#1db954]'
                : 'text-white group-hover:text-white'}"
          >
            {file.name}
          </p>
          <p class="text-xs {invalid ? 'text-red-400/60' : 'text-white/40'}">
            {#if invalid}
              Invalid file
            {:else}
              {file.bpm || 120} BPM • {#if (file.note_density || 0) < 3}Easy{:else if (file.note_density || 0) < 6}Medium{:else if (file.note_density || 0) < 10}Hard{:else}Expert{/if}
            {/if}
          </p>
        </div>

        <!-- Duration -->
        <div class="text-sm flex-shrink-0 tabular-nums flex items-center gap-1 {invalid ? 'text-red-400' : 'text-white/40'}">
          {#if invalid}
            <Icon icon="mdi:alert-circle" class="w-4 h-4" />
          {:else}
            {`${Math.floor(file.duration / 60)}:${String(Math.floor(file.duration % 60)).padStart(2, "0")}`}
          {/if}
        </div>

        <!-- Action Buttons -->
        <div class="flex items-center gap-1 flex-shrink-0">
          {#if !invalid}
            <!-- Favorite Button -->
            <button
              class="p-1.5 rounded-full transition-all {favoriteHashes.has(file.hash)
                ? 'text-[#1db954]'
                : 'text-white/30 opacity-0 group-hover:opacity-100 hover:text-white'}"
              onclick={(e) => {
                e.stopPropagation();
                handleToggleFavorite(file);
              }}
              title={favoriteHashes.has(file.hash)
                ? "Remove from favorites"
                : "Add to favorites"}
            >
              <Icon
                icon={favoriteHashes.has(file.hash) ? "mdi:heart" : "mdi:heart-outline"}
                class="w-5 h-5"
              />
            </button>

            <!-- Add to Playlist Menu -->
            <div class="relative">
              <button
                class="p-1.5 rounded-full text-white/30 opacity-0 group-hover:opacity-100 hover:text-white transition-all"
                onclick={(e) => {
                  e.stopPropagation();
                  showPlaylistMenu = showPlaylistMenu === file.path ? null : file.path;
                }}
                title="Add to playlist"
              >
                <Icon icon="mdi:playlist-plus" class="w-5 h-5" />
              </button>

            {#if showPlaylistMenu === file.path}
              <div
                class="absolute right-0 top-full mt-1 w-48 bg-[#282828] rounded-lg shadow-xl border border-white/10 py-1 z-50"
                transition:fly={{ y: -5, duration: 150 }}
              >
                <button
                  class="w-full px-3 py-2 text-left text-sm text-white/80 hover:bg-white/10 flex items-center gap-2"
                  onclick={(e) => {
                    e.stopPropagation();
                    addToQueue(file);
                    showPlaylistMenu = null;
                  }}
                >
                  <Icon icon="mdi:playlist-music" class="w-4 h-4" />
                  Add to Queue
                </button>

                {#if $savedPlaylists.length > 0}
                  <div class="border-t border-white/10 my-1"></div>
                  {#each $savedPlaylists as pl}
                    <button
                      class="w-full px-3 py-2 text-left text-sm text-white/80 hover:bg-white/10 flex items-center gap-2 truncate"
                      onclick={(e) => {
                        e.stopPropagation();
                        handleAddToPlaylist(pl.id, file);
                      }}
                    >
                      <Icon icon="mdi:playlist-music-outline" class="w-4 h-4 flex-shrink-0" />
                      <span class="truncate">{pl.name}</span>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}
          </div>
          {/if}
        </div>
      </div>
    {/each}

    <!-- Virtual scroll padding bottom -->
    <div style="height: {bottomPadding}px"></div>

    <!-- Load More Section -->
    {#if $hasMoreFiles && !$isLoadingMidi}
      <div class="py-4 flex flex-col items-center gap-2 border-t border-white/5 mt-2">
        <p class="text-xs text-white/40">
          Showing {$midiFiles.length.toLocaleString()} of {$totalMidiCount.toLocaleString()} songs
        </p>
        <div class="flex gap-2">
          <button
            class="px-4 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white text-sm font-medium transition-colors flex items-center gap-2"
            onclick={loadMoreFiles}
          >
            <Icon icon="mdi:plus" class="w-4 h-4" />
            Load 2,000 More
          </button>
          <button
            class="px-4 py-2 rounded-lg bg-white/10 hover:bg-white/15 text-white/70 text-sm font-medium transition-colors flex items-center gap-2"
            onclick={loadAllFiles}
            title="This may take a while for very large libraries"
          >
            <Icon icon="mdi:download" class="w-4 h-4" />
            Load All
          </button>
        </div>
      </div>
    {/if}

    <!-- Loading more indicator -->
    {#if $isLoadingMidi && $midiFiles.length > 0}
      <div class="py-4 flex items-center justify-center gap-2 border-t border-white/5 mt-2">
        <Icon icon="mdi:loading" class="w-5 h-5 text-[#1db954] animate-spin" />
        <span class="text-sm text-white/60">
          Loading {$midiLoadProgress.loaded.toLocaleString()} / {$midiLoadProgress.total.toLocaleString()}...
        </span>
      </div>
    {/if}
  </div>
  {/if}

  {#if $isLoadingMidi && $midiFiles.length === 0 && $midiLoadProgress.loaded === 0}
    <div
      class="flex-1 flex flex-col items-center justify-center text-white/40 py-16"
      transition:fade
    >
      <div
        class="w-20 h-20 rounded-full bg-white/5 flex items-center justify-center mb-6"
      >
        <Icon icon="mdi:loading" class="w-10 h-10 text-[#1db954] animate-spin" />
      </div>
      <p class="text-lg font-semibold mb-2 text-white/60">Loading library...</p>
      {#if $midiLoadProgress.total > 0}
        <p class="text-sm mb-3">{$midiLoadProgress.loaded.toLocaleString()} / {$midiLoadProgress.total.toLocaleString()} songs</p>
        <!-- Progress bar -->
        <div class="w-48 h-1.5 bg-white/10 rounded-full overflow-hidden">
          <div
            class="h-full bg-[#1db954] rounded-full transition-all duration-300"
            style="width: {($midiLoadProgress.loaded / $midiLoadProgress.total) * 100}%"
          ></div>
        </div>
      {:else}
        <p class="text-sm">Scanning for MIDI files...</p>
      {/if}
    </div>
  {:else if filteredFiles.length === 0 && searchQuery && !$isLoadingMidi}
    <div
      class="flex-1 flex flex-col items-center justify-center text-white/40 py-16"
      transition:fade
    >
      <div
        class="w-20 h-20 rounded-full bg-white/5 flex items-center justify-center mb-6"
      >
        <Icon icon="mdi:music-note-off" class="w-10 h-10 opacity-50" />
      </div>
      <p class="text-lg font-semibold mb-2 text-white/60">No results found</p>
      <p class="text-sm">Try a different search term</p>
    </div>
  {:else if $midiFiles.length === 0 && !$isLoadingMidi}
    <div
      class="flex-1 flex flex-col items-center justify-center text-white/40 py-16"
      transition:fade
    >
      <div
        class="w-20 h-20 rounded-full bg-white/5 flex items-center justify-center mb-6"
      >
        <Icon icon="mdi:music-note-plus" class="w-10 h-10 opacity-50" />
      </div>
      <p class="text-lg font-semibold mb-2 text-white/60">No songs yet</p>
      <p class="text-sm">Place MIDI files in the album folder</p>
    </div>
  {/if}
</div>

<!-- Toast Notification -->
{#if toast}
  <div
    class="fixed bottom-24 left-1/2 -translate-x-1/2 z-50"
    transition:fly={{ y: 20, duration: 200 }}
  >
    <div
      class="px-4 py-2 rounded-full shadow-lg flex items-center gap-2 {toast.type === 'success'
        ? 'bg-[#1db954] text-black'
        : toast.type === 'error'
          ? 'bg-red-500 text-white'
          : 'bg-white/20 text-white'}"
    >
      <Icon
        icon={toast.type === 'success' ? 'mdi:check-circle' : toast.type === 'error' ? 'mdi:alert-circle' : 'mdi:information'}
        class="w-4 h-4"
      />
      <span class="text-sm font-medium">{toast.message}</span>
    </div>
  </div>
{/if}

<!-- Import Modal -->
{#if showImportModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <!-- Backdrop -->
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { showImportModal = false; urlInput = ""; }}
    ></button>

    <!-- Modal -->
    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[400px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-white/10">
        <h3 class="text-lg font-bold">Import MIDI</h3>
        <button
          class="p-1 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors"
          onclick={() => { showImportModal = false; urlInput = ""; }}
        >
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-4 space-y-4">
        <!-- Browse Files Option -->
        <button
          class="w-full p-4 rounded-xl border-2 border-dashed border-white/20 hover:border-[#1db954] hover:bg-[#1db954]/5 transition-all group"
          onclick={openFileDialog}
        >
          <div class="flex items-center gap-4">
            <div class="w-12 h-12 rounded-xl bg-white/5 group-hover:bg-[#1db954]/20 flex items-center justify-center transition-colors">
              <Icon icon="mdi:folder-open" class="w-6 h-6 text-white/60 group-hover:text-[#1db954] transition-colors" />
            </div>
            <div class="text-left">
              <p class="font-semibold text-white group-hover:text-[#1db954] transition-colors">Browse Files</p>
              <p class="text-sm text-white/50">Select .mid files from your computer</p>
            </div>
          </div>
        </button>

        <!-- Divider -->
        <div class="flex items-center gap-3">
          <div class="flex-1 h-px bg-white/10"></div>
          <span class="text-xs text-white/40">or</span>
          <div class="flex-1 h-px bg-white/10"></div>
        </div>

        <!-- URL Input -->
        <div>
          <label class="block text-sm font-medium text-white/70 mb-2">Paste URL</label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={urlInput}
              placeholder="https://example.com/song.mid"
              class="flex-1 px-4 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent transition-all"
              onkeydown={(e) => e.key === 'Enter' && downloadFromUrl()}
              use:autofocus
            />
            <button
              class="px-4 py-2.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
              onclick={downloadFromUrl}
              disabled={isDownloading || !urlInput.trim()}
            >
              {#if isDownloading}
                <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              {:else}
                <Icon icon="mdi:download" class="w-4 h-4" />
              {/if}
              {isDownloading ? 'Downloading...' : 'Download'}
            </button>
          </div>
          <p class="text-xs text-white/40 mt-2">Works with direct .mid links from Discord, etc.</p>
        </div>
      </div>
    </div>
  </div>
{/if}

<svelte:window
  onclick={() => {
    showPlaylistMenu = null;
    showSortMenu = false;
    contextMenu = null;
  }}
/>

<SongContextMenu {contextMenu} onClose={() => contextMenu = null} />

