<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "../tauri/core-proxy.js";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { t } from "svelte-i18n";
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
    addManyToSavedPlaylist,
    createPlaylist,
    importMidiFile,
    isLoadingMidi,
    midiLoadProgress,
    totalMidiCount,
    hasMoreFiles,
    loadMoreFiles,
    loadAllFiles,
    playAllLibrary,
    libraryPlayMode,
    loadMidiFiles,
    isImportingFiles,
  } from "../stores/player.js";
  import { bandSongSelectMode, selectBandSong, cancelBandSongSelect } from "../stores/band.js";
  import SongContextMenu from "./SongContextMenu.svelte";
  import SearchSort from "./SearchSort.svelte";

  let searchQuery = "";
  let showPlaylistMenu = null;
  let toast = null;
  let toastTimeout = null;
  let isDragOver = false;
  let unlistenDrop = null;
  let unlistenHover = null;
  let unlistenCancel = null;
  let sortBy = "name-asc";

  // Import modal
  let showImportModal = false;
  let urlInput = "";
  let isDownloading = false;

  // Post-import playlist prompt
  let showPlaylistPrompt = false;
  let lastImportedFiles = [];
  let newPlaylistName = "";

  // Export library
  let isExporting = false;
  let exportProgress = { current: 0, total: 0 };

  // Context menu
  let contextMenu = null;

  // Multi-select state
  let selectedFiles = new Set(); // Set of file hashes
  let lastClickedIndex = -1; // For shift-click range selection
  let showBulkPlaylistMenu = false;
  let showCreatePlaylistModal = false;
  let createPlaylistName = "";

  // Scroll mask
  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

  // Virtual scrolling - only render visible items
  const ITEM_HEIGHT = 54; // Height of each item in pixels (52px + 2px margin)
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

  async function importFiles(midFiles, sourceName = "") {
    isImportingFiles.set(true);
    let imported = 0;
    let skipped = 0;
    let failed = 0;
    let importedFilesList = [];

    for (const filePath of midFiles) {
      const result = await importMidiFile(filePath);
      if (result.success) {
        imported++;
        importedFilesList.push(result.file);
      } else if (result.error && result.error.includes("already exists")) {
        // File already exists - find it in library and add to list
        skipped++;
        const fileName = filePath.split(/[/\\]/).pop();
        const existingFile = $midiFiles.find(f => f.name === fileName.replace(/\.mid$/i, ''));
        if (existingFile) {
          importedFilesList.push(existingFile);
        }
      } else {
        failed++;
        console.error(`Failed to import:`, result.error);
      }
    }

    isImportingFiles.set(false);
    await loadMidiFiles();

    if (imported > 0 && skipped === 0 && failed === 0) {
      showToast(`Imported ${imported} file${imported > 1 ? 's' : ''}`, "success");
    } else if (imported > 0 || skipped > 0) {
      const parts = [];
      if (imported > 0) parts.push(`${imported} imported`);
      if (skipped > 0) parts.push(`${skipped} already exist`);
      if (failed > 0) parts.push(`${failed} failed`);
      showToast(parts.join(', '), imported > 0 ? "success" : "info");
    } else {
      showToast("Failed to import files", "error");
    }

    // Show playlist prompt if multiple files were selected (imported + skipped)
    if (importedFilesList.length > 1) {
      lastImportedFiles = importedFilesList;
      newPlaylistName = sourceName || `Imported ${new Date().toLocaleDateString()}`;
      showPlaylistPrompt = true;
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

  async function openZipDialog() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "ZIP Archives", extensions: ["zip"] }],
      });

      if (selected) {
        showImportModal = false;
        await importFromZip(selected);
      }
    } catch (error) {
      console.error("Failed to open zip dialog:", error);
      showToast("Failed to open file dialog", "error");
    }
  }

  async function openFolderDialog() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });

      if (selected) {
        showImportModal = false;
        await importFromFolder(selected);
      }
    } catch (error) {
      console.error("Failed to open folder dialog:", error);
      showToast("Failed to open folder dialog", "error");
    }
  }

  async function importFromZip(zipPath) {
    isImportingFiles.set(true);
    try {
      const imported = await invoke('import_from_zip', { zipPath });
      await loadMidiFiles();
      isImportingFiles.set(false);

      if (imported.length > 0) {
        showToast(`Imported ${imported.length} file${imported.length > 1 ? 's' : ''} from zip`, "success");
        // Show playlist prompt
        if (imported.length > 1) {
          lastImportedFiles = imported;
          const zipName = zipPath.split(/[\\/]/).pop()?.replace('.zip', '') || 'Imported';
          newPlaylistName = zipName;
          showPlaylistPrompt = true;
        }
      } else {
        showToast("No MIDI files found in zip", "info");
      }
    } catch (error) {
      isImportingFiles.set(false);
      console.error("Failed to import from zip:", error);
      showToast(error.toString(), "error");
    }
  }

  async function importFromFolder(folderPath) {
    isImportingFiles.set(true);
    try {
      // Get list of midi files in folder
      const midiPaths = await invoke('list_midi_in_folder', { folderPath });

      if (midiPaths.length === 0) {
        isImportingFiles.set(false);
        showToast("No MIDI files found in folder", "info");
        return;
      }

      // Import using existing function
      const folderName = folderPath.split(/[\\/]/).pop() || 'Imported';
      await importFiles(midiPaths, folderName);
    } catch (error) {
      isImportingFiles.set(false);
      console.error("Failed to import from folder:", error);
      showToast(error.toString(), "error");
    }
  }

  function createPlaylistFromImport() {
    if (lastImportedFiles.length > 0 && newPlaylistName.trim()) {
      const id = createPlaylist(newPlaylistName.trim());
      addManyToSavedPlaylist(id, lastImportedFiles);
      showToast($t("library.createdPlaylist", { values: { name: newPlaylistName.trim() } }), "success");
    }
    showPlaylistPrompt = false;
    lastImportedFiles = [];
    newPlaylistName = "";
  }

  function skipPlaylistPrompt() {
    showPlaylistPrompt = false;
    lastImportedFiles = [];
    newPlaylistName = "";
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
        showToast($t("library.addedSongsToQueue", { values: { count: count.toLocaleString() } }) + " (exported)", "success");
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
    showToast(added ? $t("library.addedToQueue") : $t("library.alreadyInQueue"), added ? "success" : "info");
  }

  function handleAddToPlaylist(playlistId, file) {
    const pl = $savedPlaylists.find(p => p.id === playlistId);
    const alreadyExists = pl?.tracks.some(t => t.path === file.path);
    addToSavedPlaylist(playlistId, file);
    showPlaylistMenu = null;
    showToast(
      alreadyExists ? $t("library.alreadyInQueue") : $t("library.addedSongsToPlaylist", { values: { count: 1, name: pl?.name } }),
      alreadyExists ? "info" : "success"
    );
  }

  function handleToggleFavorite(file) {
    const wasFavorite = $favorites.some((f) => f.hash === file.hash);
    toggleFavorite(file);
    showToast(
      wasFavorite ? $t("library.removedFromFavorites") : $t("library.addedToFavorites"),
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

  // Multi-select functions
  function handleFileClick(e, file, index) {
    const invalid = isInvalidFile(file);
    if (invalid) return;

    // Ctrl/Cmd + Click: Toggle individual selection
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      selectedFiles = new Set(selectedFiles);
      if (selectedFiles.has(file.hash)) {
        selectedFiles.delete(file.hash);
      } else {
        selectedFiles.add(file.hash);
      }
      lastClickedIndex = index;
      return;
    }

    // Shift + Click: Range selection
    if (e.shiftKey && lastClickedIndex !== -1) {
      e.preventDefault();
      const start = Math.min(lastClickedIndex, index);
      const end = Math.max(lastClickedIndex, index);
      selectedFiles = new Set(selectedFiles);
      for (let i = start; i <= end; i++) {
        const f = filteredFiles[i];
        if (f && !isInvalidFile(f)) {
          selectedFiles.add(f.hash);
        }
      }
      return;
    }

    // Normal click: Clear selection and play
    if (selectedFiles.size > 0) {
      // If clicking on a selected file, keep selection
      // Otherwise clear and handle normally
      if (!selectedFiles.has(file.hash)) {
        clearSelection();
      }
    }
    lastClickedIndex = index;
  }

  function clearSelection() {
    selectedFiles = new Set();
    lastClickedIndex = -1;
    showBulkPlaylistMenu = false;
  }

  function selectAll() {
    selectedFiles = new Set(
      filteredFiles
        .filter(f => !isInvalidFile(f))
        .map(f => f.hash)
    );
  }

  function getSelectedFileObjects() {
    return filteredFiles.filter(f => selectedFiles.has(f.hash));
  }

  function addSelectedToQueue() {
    const files = getSelectedFileObjects();
    playlist.update((list) => {
      const existingPaths = new Set(list.map(f => f.path));
      const newFiles = files.filter(f => !existingPaths.has(f.path));
      return [...list, ...newFiles];
    });
    showToast($t("library.addedSongsToQueue", { values: { count: files.length } }), "success");
    clearSelection();
  }

  function addSelectedToPlaylist(playlistId) {
    const files = getSelectedFileObjects();
    const pl = $savedPlaylists.find(p => p.id === playlistId);
    addManyToSavedPlaylist(playlistId, files);
    showBulkPlaylistMenu = false;
    showToast($t("library.addedSongsToPlaylist", { values: { count: files.length, name: pl?.name } }), "success");
    clearSelection();
  }

  function openCreatePlaylistModal() {
    createPlaylistName = `Playlist ${$savedPlaylists.length + 1}`;
    showCreatePlaylistModal = true;
    showBulkPlaylistMenu = false;
  }

  function createPlaylistFromSelected() {
    if (!createPlaylistName.trim()) return;
    const files = getSelectedFileObjects();
    const id = createPlaylist(createPlaylistName.trim());
    addManyToSavedPlaylist(id, files);
    showToast($t("library.createdPlaylistWithSongs", { values: { name: createPlaylistName.trim(), count: files.length } }), "success");
    showCreatePlaylistModal = false;
    createPlaylistName = "";
    clearSelection();
  }

  // Context menu
  function handleContextMenu(e, file) {
    e.preventDefault();
    // If right-clicking on a selected file, keep selection for bulk context menu
    // Otherwise, clear selection
    if (selectedFiles.size > 0 && !selectedFiles.has(file.hash)) {
      clearSelection();
    }
    contextMenu = { x: e.clientX, y: e.clientY, file };
  }

  // Make sortOptions reactive so labels update on language change
  $: sortOptions = [
    { id: "name-asc", label: $t("sort.nameAsc"), icon: "mdi:sort-alphabetical-ascending" },
    { id: "name-desc", label: $t("sort.nameDesc"), icon: "mdi:sort-alphabetical-descending" },
    { id: "duration-asc", label: $t("sort.durationAsc"), icon: "mdi:sort-numeric-ascending" },
    { id: "duration-desc", label: $t("sort.durationDesc"), icon: "mdi:sort-numeric-descending" },
    { id: "bpm-asc", label: $t("sort.bpmAsc"), icon: "mdi:speedometer-slow" },
    { id: "bpm-desc", label: $t("sort.bpmDesc"), icon: "mdi:speedometer" },
    { id: "density-asc", label: $t("sort.difficultyAsc"), icon: "mdi:music-note" },
    { id: "density-desc", label: $t("sort.difficultyDesc"), icon: "mdi:music-note-plus" },
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
        <p class="text-lg font-semibold text-[#1db954]">{$t("library.dropFilesHere")}</p>
        <p class="text-sm text-white/60">{$t("nav.library")}</p>
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
      <p class="text-sm flex-1">{$t("band.selectSong")}</p>
      <button
        class="text-xs text-white/50 hover:text-white transition-colors"
        onclick={cancelBandSongSelect}
      >
        {$t("band.cancelSelection")}
      </button>
    </div>
  {/if}

  <!-- Header -->
  <div class="mb-4">
    <div class="flex items-center justify-between mb-2">
      <h2 class="text-2xl font-bold">{$t("library.title")}</h2>
      <div class="flex items-center gap-2">
        <!-- Play All / Shuffle All buttons -->
        {#if filteredFiles.length > 0 && !$isLoadingMidi}
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-[#1db954] hover:bg-[#1ed760] text-white text-sm font-medium transition-all"
            onclick={() => {
              playAllLibrary(filteredFiles, 0, false);
              showToast($t("library.playingSongs", { values: { count: filteredFiles.length.toLocaleString() } }), "success");
            }}
            title={$t("library.playAll")}
          >
            <Icon icon="mdi:play" class="w-4 h-4" />
            {$t("library.playAll")}
          </button>
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white/80 hover:text-white text-sm font-medium transition-all"
            onclick={() => {
              playAllLibrary(filteredFiles, 0, true);
              showToast($t("library.shufflingSongs", { values: { count: filteredFiles.length.toLocaleString() } }), "success");
            }}
            title={$t("library.shuffleAll")}
          >
            <Icon icon="mdi:shuffle" class="w-4 h-4" />
            {$t("library.shuffleAll")}
          </button>
        {/if}
        {#if $midiFiles.length > 0 && !$isLoadingMidi}
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white/80 hover:text-white text-sm font-medium transition-all disabled:opacity-50"
            onclick={exportLibrary}
            disabled={isExporting}
            title={$t("playlists.export")}
          >
            {#if isExporting}
              <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              {exportProgress.total > 0 ? `${Math.round(exportProgress.current / exportProgress.total * 100)}%` : $t("playlists.export")}
            {:else}
              <Icon icon="mdi:export" class="w-4 h-4" />
              {$t("playlists.export")}
            {/if}
          </button>
        {/if}
        <button
          class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white/80 hover:text-white text-sm font-medium transition-all"
          onclick={() => showImportModal = true}
          title={$t("playlists.import")}
        >
          <Icon icon="mdi:plus" class="w-4 h-4" />
          {$t("playlists.import")}
        </button>
      </div>
    </div>
    <p class="text-sm text-white/60 mb-4 flex items-center gap-2">
      {#if $isLoadingMidi}
        <span class="flex items-center gap-2">
          <Icon icon="mdi:loading" class="w-4 h-4 animate-spin text-[#1db954]" />
          {#if $midiLoadProgress.total > 0}
            {$t("common.loading")} {$midiLoadProgress.loaded.toLocaleString()} / {$midiLoadProgress.total.toLocaleString()}
          {:else}
            {$t("common.loading")}
          {/if}
        </span>
      {:else}
        <span>{filteredFiles.length} {$t("library.of")} {$midiFiles.length} {$t("library.songs")}</span>
        <span class="text-white/30">•</span>
        <span class="text-xs text-white/30 flex items-center gap-1">
          <Icon icon="mdi:mouse" class="w-3 h-3" />
          {$t("library.selectHint")}
        </span>
      {/if}
    </p>

    <!-- Search Input with Sort -->
    <SearchSort
      bind:searchQuery
      bind:sortBy
      placeholder={$t("library.searchPlaceholder")}
      {sortOptions}
    />
  </div>

  <!-- Selection Toolbar -->
  {#if selectedFiles.size > 0}
    <div
      class="mb-3 p-3 rounded-lg bg-[#1db954]/10 border border-[#1db954]/30 flex items-center gap-3"
      transition:fly={{ y: -10, duration: 200 }}
    >
      <div class="flex items-center gap-2 flex-1">
        <Icon icon="mdi:checkbox-multiple-marked" class="w-5 h-5 text-[#1db954]" />
        <span class="text-sm font-medium">{selectedFiles.size} {$t("library.selected")}</span>
        <button
          class="text-xs text-white/50 hover:text-white transition-colors underline"
          onclick={selectAll}
        >
          {$t("library.selectAll")} ({filteredFiles.filter(f => !isInvalidFile(f)).length})
        </button>
      </div>
      <div class="flex items-center gap-2">
        <!-- Add to Queue -->
        <button
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white/80 hover:text-white text-sm font-medium transition-all"
          onclick={addSelectedToQueue}
          title={$t("library.addToQueue")}
        >
          <Icon icon="mdi:playlist-plus" class="w-4 h-4" />
          {$t("library.queue")}
        </button>

        <!-- Add to Playlist Dropdown -->
        <div class="relative">
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-[#1db954] hover:bg-[#1ed760] text-white text-sm font-medium transition-all"
            onclick={(e) => {
              e.stopPropagation();
              showBulkPlaylistMenu = !showBulkPlaylistMenu;
            }}
            title={$t("library.addToPlaylist")}
          >
            <Icon icon="mdi:playlist-music" class="w-4 h-4" />
            {$t("library.addToPlaylistBtn")}
            <Icon icon="mdi:chevron-down" class="w-4 h-4" />
          </button>

          {#if showBulkPlaylistMenu}
            <div
              class="absolute right-0 top-full mt-1 w-48 bg-[#282828] rounded-lg shadow-xl border border-white/10 py-1 z-50"
              transition:fly={{ y: -5, duration: 150 }}
              onclick={(e) => e.stopPropagation()}
            >
              <!-- Create New Playlist -->
              <button
                class="w-full px-3 py-2 text-left text-sm text-[#1db954] hover:bg-white/10 flex items-center gap-2"
                onclick={openCreatePlaylistModal}
              >
                <Icon icon="mdi:playlist-plus" class="w-4 h-4 flex-shrink-0" />
                <span>{$t("library.newPlaylist")}</span>
              </button>
              {#if $savedPlaylists.length > 0}
                <div class="border-t border-white/10 my-1"></div>
                <div class="max-h-48 overflow-y-auto">
                  {#each $savedPlaylists as pl}
                    <button
                      class="w-full px-3 py-2 text-left text-sm text-white/80 hover:bg-white/10 flex items-center gap-2 truncate"
                      onclick={() => addSelectedToPlaylist(pl.id)}
                    >
                      <Icon icon="mdi:playlist-music-outline" class="w-4 h-4 flex-shrink-0" />
                      <span class="truncate">{pl.name}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Clear Selection -->
        <button
          class="p-1.5 rounded-full text-white/50 hover:text-white hover:bg-white/10 transition-all"
          onclick={clearSelection}
          title={$t("library.clearSelection")}
        >
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>
    </div>
  {/if}

  <!-- Song List (Scrollable) - show if we have files, even while loading more -->
  {#if $midiFiles.length > 0}
  <div
    bind:this={scrollContainer}
    onscroll={handleScroll}
    class="flex-1 overflow-y-auto pr-2 -mx-1 px-1 {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
  >
    <!-- Virtual scroll padding top -->
    <div style="height: {topPadding}px"></div>

    {#each visibleFiles as file, i (`${file.path}-${i}`)}
      {@const invalid = isInvalidFile(file)}
      {@const index = visibleStartIndex + i}
      {@const isSelected = selectedFiles.has(file.hash)}
      <div
        class="group spotify-list-item flex items-center gap-4 py-2 rounded-lg transition-colors duration-150 {isSelected
          ? 'bg-[#1db954]/20 ring-1 ring-[#1db954]/30'
          : $currentFile === file.path
            ? 'bg-white/10 ring-1 ring-white/5'
            : 'hover:bg-white/5'} {invalid ? 'opacity-60' : ''}"
        style="height: {ITEM_HEIGHT}px; margin-bottom: 2px;"
        title={invalid ? $t("library.invalidFile") : $t("library.selectHint")}
        oncontextmenu={(e) => handleContextMenu(e, file)}
        onclick={(e) => handleFileClick(e, file, index)}
      >
        <!-- Number / Checkbox / Play Button / Playing Indicator -->
        <div class="w-8 flex items-center justify-center flex-shrink-0">
          {#if isSelected}
            <!-- Checkbox for selected state -->
            <button
              class="flex items-center justify-center w-6 h-6 rounded bg-[#1db954] transition-transform hover:scale-110"
              onclick={(e) => {
                e.stopPropagation();
                selectedFiles = new Set(selectedFiles);
                selectedFiles.delete(file.hash);
              }}
              title={$t("library.deselect")}
            >
              <Icon icon="mdi:check" class="w-4 h-4 text-black" />
            </button>
          {:else if selectedFiles.size > 0 && !invalid}
            <!-- Empty checkbox when in selection mode -->
            <button
              class="flex items-center justify-center w-6 h-6 rounded border-2 border-white/30 hover:border-[#1db954] transition-colors group-hover:border-white/50"
              onclick={(e) => {
                e.stopPropagation();
                selectedFiles = new Set(selectedFiles);
                selectedFiles.add(file.hash);
              }}
              title={$t("library.select")}
            >
            </button>
          {:else if $currentFile === file.path && $isPlaying && !$isPaused}
            <!-- Playing indicator (animated bars) -->
            <div class="flex items-end gap-0.5 h-4">
                    <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-1" style="height: 60%;"></div>
                    <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-2" style="height: 100%;"></div>
                    <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-3" style="height: 80%;"></div>
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
                onclick={(e) => {
                  e.stopPropagation();
                  handlePlay(file);
                }}
                title={$bandSongSelectMode ? $t("library.selectForBand") : $t("player.play")}
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
          onclick={(e) => {
            if (invalid) return;
            // Don't play if Ctrl/Shift held (selection mode) or if selection exists
            if (e.ctrlKey || e.metaKey || e.shiftKey) return;
            if (selectedFiles.size > 0 && !isSelected) return;
            if (selectedFiles.size === 0) {
              handlePlay(file);
            }
          }}
          onkeydown={(event) => {
            if (!invalid && (event.key === "Enter" || event.key === " ")) {
              event.preventDefault();
              if (selectedFiles.size === 0) {
                handlePlay(file);
              }
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
              {$t("library.invalidFile")}
            {:else}
              {file.bpm || 120} BPM • {#if (file.note_density || 0) < 3}{$t("library.easy")}{:else if (file.note_density || 0) < 6}{$t("library.medium")}{:else if (file.note_density || 0) < 10}{$t("library.hard")}{:else}{$t("library.expert")}{/if}
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
                ? $t("library.removeFromFavorites")
                : $t("library.addToFavorites")}
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
                title={$t("library.addToPlaylist")}
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
                  {$t("library.addToQueue")}
                </button>

                {#if $savedPlaylists.length > 0}
                  <div class="border-t border-white/10 my-1"></div>
                  <div class="max-h-48 overflow-y-auto">
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
                  </div>
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
          {$t("library.showingOf", { values: { loaded: $midiFiles.length.toLocaleString(), total: $totalMidiCount.toLocaleString() } })}
        </p>
        <div class="flex gap-2">
          <button
            class="px-4 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white text-sm font-medium transition-colors flex items-center gap-2"
            onclick={loadMoreFiles}
          >
            <Icon icon="mdi:plus" class="w-4 h-4" />
            {$t("library.loadMore")}
          </button>
          <button
            class="px-4 py-2 rounded-lg bg-white/10 hover:bg-white/15 text-white/70 text-sm font-medium transition-colors flex items-center gap-2"
            onclick={loadAllFiles}
            title={$t("library.loadAll")}
          >
            <Icon icon="mdi:download" class="w-4 h-4" />
            {$t("library.loadAll")}
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
      <p class="text-lg font-semibold mb-2 text-white/60">{$t("library.loadingLibrary")}</p>
      {#if $midiLoadProgress.total > 0}
        <p class="text-sm mb-3">{$midiLoadProgress.loaded.toLocaleString()} / {$midiLoadProgress.total.toLocaleString()} {$t("library.songs")}</p>
        <!-- Progress bar -->
        <div class="w-48 h-1.5 bg-white/10 rounded-full overflow-hidden">
          <div
            class="h-full bg-[#1db954] rounded-full transition-all duration-300"
            style="width: {($midiLoadProgress.loaded / $midiLoadProgress.total) * 100}%"
          ></div>
        </div>
      {:else}
        <p class="text-sm">{$t("library.scanningFiles")}</p>
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
      <p class="text-lg font-semibold mb-2 text-white/60">{$t("library.noResultsFound")}</p>
      <p class="text-sm">{$t("library.tryDifferentSearch")}</p>
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
      <p class="text-lg font-semibold mb-2 text-white/60">{$t("library.noSongsYet")}</p>
      <p class="text-sm">{$t("library.placeMidiFiles")}</p>
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
        <h3 class="text-lg font-bold">{$t("modals.import.title")}</h3>
        <button
          class="p-1 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors"
          onclick={() => { showImportModal = false; urlInput = ""; }}
        >
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-4 space-y-3">
        <!-- Browse Files Option -->
        <button
          class="w-full p-3 rounded-xl border-2 border-dashed border-white/20 hover:border-[#1db954] hover:bg-[#1db954]/5 transition-all group"
          onclick={openFileDialog}
        >
          <div class="flex items-center gap-4">
            <div class="w-10 h-10 rounded-xl bg-white/5 group-hover:bg-[#1db954]/20 flex items-center justify-center transition-colors">
              <Icon icon="mdi:file-music" class="w-5 h-5 text-white/60 group-hover:text-[#1db954] transition-colors" />
            </div>
            <div class="text-left">
              <p class="font-semibold text-white group-hover:text-[#1db954] transition-colors">{$t("modals.import.midiFiles")}</p>
              <p class="text-xs text-white/50">{$t("modals.import.selectMidFiles")}</p>
            </div>
          </div>
        </button>

        <!-- Two column options -->
        <div class="grid grid-cols-2 gap-3">
          <!-- Import Zip Option -->
          <button
            class="p-3 rounded-xl border-2 border-dashed border-white/20 hover:border-[#1db954] hover:bg-[#1db954]/5 transition-all group"
            onclick={openZipDialog}
          >
            <div class="flex flex-col items-center gap-2 text-center">
              <div class="w-10 h-10 rounded-xl bg-white/5 group-hover:bg-[#1db954]/20 flex items-center justify-center transition-colors">
                <Icon icon="mdi:folder-zip" class="w-5 h-5 text-white/60 group-hover:text-[#1db954] transition-colors" />
              </div>
              <div>
                <p class="font-semibold text-sm text-white group-hover:text-[#1db954] transition-colors">{$t("modals.import.zipFile")}</p>
                <p class="text-xs text-white/50">{$t("modals.import.extractMid")}</p>
              </div>
            </div>
          </button>

          <!-- Browse Folder Option -->
          <button
            class="p-3 rounded-xl border-2 border-dashed border-white/20 hover:border-[#1db954] hover:bg-[#1db954]/5 transition-all group"
            onclick={openFolderDialog}
          >
            <div class="flex flex-col items-center gap-2 text-center">
              <div class="w-10 h-10 rounded-xl bg-white/5 group-hover:bg-[#1db954]/20 flex items-center justify-center transition-colors">
                <Icon icon="mdi:folder-open" class="w-5 h-5 text-white/60 group-hover:text-[#1db954] transition-colors" />
              </div>
              <div>
                <p class="font-semibold text-sm text-white group-hover:text-[#1db954] transition-colors">{$t("modals.import.folder")}</p>
                <p class="text-xs text-white/50">{$t("modals.import.scanForMid")}</p>
              </div>
            </div>
          </button>
        </div>

        <!-- Divider -->
        <div class="flex items-center gap-3">
          <div class="flex-1 h-px bg-white/10"></div>
          <span class="text-xs text-white/40">{$t("modals.import.orPasteUrl")}</span>
          <div class="flex-1 h-px bg-white/10"></div>
        </div>

        <!-- URL Input -->
        <div>
          <label class="block text-sm font-medium text-white/70 mb-2">{$t("modals.import.pasteUrl")}</label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={urlInput}
              placeholder={$t("modals.import.urlPlaceholder")}
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
              {isDownloading ? $t("modals.import.downloading") : $t("modals.import.download")}
            </button>
          </div>
          <p class="text-xs text-white/40 mt-2">{$t("modals.import.urlHint")}</p>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Playlist Creation Prompt -->
{#if showPlaylistPrompt}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <button
      class="absolute inset-0 bg-black/60"
      onclick={skipPlaylistPrompt}
    ></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[360px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <div class="p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-[#1db954]/20 flex items-center justify-center mx-auto mb-3">
          <Icon icon="mdi:playlist-plus" class="w-6 h-6 text-[#1db954]" />
        </div>
        <h3 class="text-lg font-bold mb-2">{$t("modals.createPlaylist.createFromImport")}</h3>
        <p class="text-sm text-white/60 mb-4">
          {$t("modals.createPlaylist.songsImported", { values: { count: lastImportedFiles.length } })}
        </p>
        <input
          type="text"
          bind:value={newPlaylistName}
          placeholder={$t("modals.createPlaylist.playlistName")}
          class="w-full px-4 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent transition-all mb-4"
          onkeydown={(e) => e.key === 'Enter' && createPlaylistFromImport()}
        />
      </div>

      <div class="flex gap-2 p-4 pt-0">
        <button
          class="flex-1 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition-colors"
          onclick={skipPlaylistPrompt}
        >
          {$t("modals.createPlaylist.skip")}
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors disabled:opacity-50"
          onclick={createPlaylistFromImport}
          disabled={!newPlaylistName.trim()}
        >
          {$t("common.create")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Create Playlist from Selection Modal -->
{#if showCreatePlaylistModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { showCreatePlaylistModal = false; createPlaylistName = ""; }}
    ></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[360px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <div class="p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-[#1db954]/20 flex items-center justify-center mx-auto mb-3">
          <Icon icon="mdi:playlist-plus" class="w-6 h-6 text-[#1db954]" />
        </div>
        <h3 class="text-lg font-bold mb-2">{$t("modals.createPlaylist.title")}</h3>
        <p class="text-sm text-white/60 mb-4">
          {$t("modals.createPlaylist.songsSelected", { values: { count: selectedFiles.size } })}
        </p>
        <input
          type="text"
          bind:value={createPlaylistName}
          placeholder={$t("modals.createPlaylist.playlistName")}
          class="w-full px-4 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent transition-all mb-4"
          onkeydown={(e) => e.key === 'Enter' && createPlaylistFromSelected()}
        />
      </div>

      <div class="flex gap-2 p-4 pt-0">
        <button
          class="flex-1 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition-colors"
          onclick={() => { showCreatePlaylistModal = false; createPlaylistName = ""; }}
        >
          {$t("common.cancel")}
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors disabled:opacity-50"
          onclick={createPlaylistFromSelected}
          disabled={!createPlaylistName.trim()}
        >
          {$t("common.create")}
        </button>
      </div>
    </div>
  </div>
{/if}

<svelte:window
  onclick={() => {
    showPlaylistMenu = null;
    showBulkPlaylistMenu = false;
    contextMenu = null;
  }}
  onkeydown={(e) => {
    // Escape to clear selection
    if (e.key === 'Escape' && selectedFiles.size > 0) {
      clearSelection();
    }
    // Ctrl+A to select all when focused in library
    if ((e.ctrlKey || e.metaKey) && e.key === 'a' && scrollContainer?.contains(document.activeElement)) {
      e.preventDefault();
      selectAll();
    }
  }}
/>

<SongContextMenu {contextMenu} onClose={() => contextMenu = null} />

