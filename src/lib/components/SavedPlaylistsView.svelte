<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { onMount } from "svelte";
  import { dndzone } from "svelte-dnd-action";
  import { invoke } from "../tauri/core-proxy.js";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { t } from "svelte-i18n";
  import {
    savedPlaylists,
    midiFiles,
    loadMidiFiles,
    createPlaylist,
    deletePlaylist,
    renamePlaylist,
    loadPlaylistToQueue,
    reorderPlaylists,
    removeFromSavedPlaylist,
    reorderSavedPlaylist,
    setPlaylistTracks,
    setPlaylistsOrder,
    addManyToSavedPlaylist,
    playMidi,
    playlist,
    activePlaylistId,
    currentFile,
    isPlaying,
    isPaused,
  } from "../stores/player.js";
  import SongContextMenu from "./SongContextMenu.svelte";
  import SearchSort from "./SearchSort.svelte";

  // Search & Sort for tracks
  let searchQuery = "";
  let sortBy = "manual";

  $: sortOptions = [
    { id: "manual", label: $t("sort.manual"), icon: "mdi:drag" },
    { id: "name-asc", label: $t("sort.nameAsc"), icon: "mdi:sort-alphabetical-ascending" },
    { id: "name-desc", label: $t("sort.nameDesc"), icon: "mdi:sort-alphabetical-descending" },
    { id: "duration-asc", label: $t("sort.durationAsc"), icon: "mdi:sort-numeric-ascending" },
    { id: "duration-desc", label: $t("sort.durationDesc"), icon: "mdi:sort-numeric-descending" },
  ];

  // Context menu
  let contextMenu = null;
  let isExporting = false;
  let isImporting = false;

  function handleContextMenu(e, file) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, file };
  }

  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

  function handleScroll(e) {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    showTopMask = scrollTop > 10;
    showBottomMask = scrollTop + clientHeight < scrollHeight - 10;
  }

  function checkScrollMask() {
    setTimeout(() => {
      if (scrollContainer) {
        const { scrollHeight, clientHeight } = scrollContainer;
        showBottomMask = scrollHeight > clientHeight;
        showTopMask = false;
      }
    }, 100);
  }

  onMount(() => {
    checkScrollMask();
  });

  // Re-check scroll mask when switching views
  $: if (selectedPlaylistId !== undefined) {
    checkScrollMask();
  }

  let showCreateModal = false;
  let newPlaylistName = "";
  let editingPlaylistId = null;
  let editingName = "";
  let selectedPlaylistId = null;
  let deleteConfirmId = null; // For delete confirmation modal
  const flipDurationMs = 200;

  // Get selected playlist reactively from store
  $: selectedPlaylist = selectedPlaylistId
    ? $savedPlaylists.find((p) => p.id === selectedPlaylistId)
    : null;

  // Filter and sort tracks
  $: filteredTracks = (() => {
    if (!selectedPlaylist) return [];
    let result = searchQuery.trim()
      ? selectedPlaylist.tracks.filter(t => t.name.toLowerCase().includes(searchQuery.toLowerCase()))
      : [...selectedPlaylist.tracks];

    if (sortBy !== "manual") {
      result.sort((a, b) => {
        switch (sortBy) {
          case "name-asc": return a.name.localeCompare(b.name);
          case "name-desc": return b.name.localeCompare(a.name);
          case "duration-asc": return (a.duration || 0) - (b.duration || 0);
          case "duration-desc": return (b.duration || 0) - (a.duration || 0);
          default: return 0;
        }
      });
    }
    return result;
  })();

  // Track items for drag and drop in playlist detail view
  $: trackItems = filteredTracks.map((track, index) => ({
    ...track,
    id: `${track.hash}-${index}`, // Combine hash with index for unique ID (allows duplicates)
    originalIndex: index,
  }));

  // Items for drag and drop
  $: items = $savedPlaylists.map((pl, index) => ({
    ...pl,
    originalIndex: index,
  }));

  function handleCreate() {
    if (newPlaylistName.trim()) {
      createPlaylist(newPlaylistName.trim());
      newPlaylistName = "";
      showCreateModal = false;
    }
  }

  function startEditing(playlist) {
    editingPlaylistId = playlist.id;
    editingName = playlist.name;
  }

  function saveEdit() {
    if (editingName.trim() && editingPlaylistId) {
      renamePlaylist(editingPlaylistId, editingName.trim());
    }
    editingPlaylistId = null;
    editingName = "";
  }

  function handleDelete(id) {
    // Show confirmation modal instead of native confirm
    deleteConfirmId = id;
  }

  function confirmDelete() {
    if (deleteConfirmId) {
      deletePlaylist(deleteConfirmId);
      if (selectedPlaylistId === deleteConfirmId) {
        selectedPlaylistId = null;
      }
      deleteConfirmId = null;
    }
  }

  function cancelDelete() {
    deleteConfirmId = null;
  }

  function handleDndConsider(e) {
    isPlaylistDragging = true;
    items = e.detail.items;
  }

  function handleDndFinalize(e) {
    isPlaylistDragging = false;
    const newItems = e.detail.items;
    // Update the store with new order and persist to file
    const newOrder = newItems.map(({ originalIndex, ...pl }) => pl);
    setPlaylistsOrder(newOrder);
    items = newItems.map((item, index) => ({
      ...item,
      originalIndex: index,
    }));
  }

  async function handleLoadToQueue(playlist) {
    await loadPlaylistToQueue(playlist.id);
  }

  function goBack() {
    selectedPlaylistId = null;
  }

  // Track management functions
  let isTrackDragging = false;
  let isPlaylistDragging = false;

  function handleTrackDndConsider(e) {
    isTrackDragging = true;
    trackItems = e.detail.items;
  }

  function handleTrackDndFinalize(e) {
    isTrackDragging = false;
    if (!selectedPlaylistId) return;

    const newItems = e.detail.items;
    const newTracks = newItems.map(({ id, originalIndex, ...track }) => track);
    setPlaylistTracks(selectedPlaylistId, newTracks);
  }

  function handleRemoveTrack(trackHash) {
    if (selectedPlaylistId) {
      removeFromSavedPlaylist(selectedPlaylistId, trackHash);
    }
  }

  async function handlePlayTrack(track) {
    // Add track to queue and play
    playlist.update((list) => {
      if (!list.find((f) => f.path === track.path)) {
        return [...list, track];
      }
      return list;
    });
    await playMidi(track.path);
  }

  async function exportPlaylist(pl = null) {
    const targetPlaylist = pl || selectedPlaylist;
    if (!targetPlaylist || targetPlaylist.tracks.length === 0 || isExporting) return;

    try {
      isExporting = true;
      const safeFilename = targetPlaylist.name.replace(/[<>:"/\\|?*]/g, "_");
      const exportPath = await save({
        title: "Export Playlist",
        defaultPath: `${safeFilename}.zip`,
        filters: [{ name: "Zip Archive", extensions: ["zip"] }],
      });

      if (exportPath) {
        // Hydrate tracks with paths from midiFiles using hash
        const filesByHash = new Map($midiFiles.map(f => [f.hash, f]));
        const hydratedTracks = targetPlaylist.tracks
          .map(track => filesByHash.get(track.hash) || track)
          .filter(track => track.path); // Only include files with valid paths

        await invoke("export_playlist", {
          playlistName: targetPlaylist.name,
          tracks: hydratedTracks,
          exportPath,
        });
      }
    } catch (error) {
      console.error("Failed to export playlist:", error);
    } finally {
      isExporting = false;
    }
  }

  async function importPlaylist() {
    if (isImporting) return;

    try {
      isImporting = true;
      const zipPath = await open({
        title: "Import Playlist",
        filters: [{ name: "Zip Archive", extensions: ["zip"] }],
      });

      if (zipPath) {
        const result = await invoke("import_zip", { zipPath });

        // Reload library to include new files
        await loadMidiFiles();

        // Create a new playlist with imported files
        if (result.imported_files.length > 0) {
          const playlistName = result.export_type === "playlist" ? result.name : "Imported";
          const newPlaylistId = createPlaylist(playlistName);

          // Add all imported files at once (single save to avoid race condition)
          addManyToSavedPlaylist(newPlaylistId, result.imported_files);

          // Select the new playlist
          selectedPlaylistId = newPlaylistId;
        }
      }
    } catch (error) {
      console.error("Failed to import:", error);
    } finally {
      isImporting = false;
    }
  }
</script>

<div class="h-full flex flex-col">
  {#if selectedPlaylist}
    <!-- Playlist Detail View -->
    <div class="mb-3" in:fly={{ x: 20, duration: 200 }}>
      <div class="flex items-center gap-3">
        <button
          class="p-1 -ml-1 text-white/60 hover:text-white transition-colors"
          onclick={goBack}
          title={$t("playlists.backToPlaylists")}
        >
          <Icon icon="mdi:arrow-left" class="w-5 h-5" />
        </button>

        <div
          class="w-10 h-10 rounded bg-white/10 flex items-center justify-center flex-shrink-0"
        >
          <Icon icon="mdi:playlist-music" class="w-5 h-5 text-[#1db954]" />
        </div>

        <div class="flex-1 min-w-0">
          {#if editingPlaylistId === selectedPlaylist.id}
            <input
              type="text"
              bind:value={editingName}
              class="bg-white/10 border border-white/20 rounded px-2 py-0.5 text-base font-bold w-full"
              onblur={saveEdit}
              onkeydown={(e) => e.key === "Enter" && saveEdit()}
              autofocus
            />
          {:else}
            <h2
              class="text-lg font-bold truncate cursor-pointer hover:text-[#1db954] transition-colors"
              onclick={() => startEditing(selectedPlaylist)}
              title={$t("playlists.clickToRename")}
            >
              {selectedPlaylist.name}
            </h2>
          {/if}
          <p class="text-xs text-white/50">
            {selectedPlaylist.tracks.length} {$t("library.songs")}
          </p>
        </div>

        <div class="flex items-center gap-1.5 flex-shrink-0">
          <button
            class="spotify-button spotify-button--primary flex items-center gap-1.5 text-sm py-1.5 px-3 !text-white"
            onclick={() => handleLoadToQueue(selectedPlaylist)}
            disabled={selectedPlaylist.tracks.length === 0}
          >
            <Icon icon="mdi:play" class="w-4 h-4" />
            {$t("playlists.play")}
          </button>
          <button
            class="p-2 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-all"
            onclick={exportPlaylist}
            disabled={selectedPlaylist.tracks.length === 0 || isExporting}
            title={$t("playlists.exportPlaylist")}
          >
            <Icon icon={isExporting ? "mdi:loading" : "mdi:export"} class="w-4 h-4 {isExporting ? 'animate-spin' : ''}" />
          </button>
          <button
            class="p-2 rounded-full hover:bg-red-500/20 text-white/60 hover:text-red-400 transition-all"
            onclick={() => handleDelete(selectedPlaylist.id)}
            title={$t("playlists.deletePlaylist")}
          >
            <Icon icon="mdi:delete-outline" class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- Search + Sort -->
    {#if selectedPlaylist.tracks.length > 0}
      <div class="mb-4">
        <SearchSort
          bind:searchQuery
          bind:sortBy
          placeholder={$t("playlists.searchTracks")}
          {sortOptions}
        />
      </div>
    {/if}

    <!-- Playlist Tracks with Drag and Drop -->
    <div
      bind:this={scrollContainer}
      onscroll={handleScroll}
      class="flex-1 overflow-y-auto pr-2 {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
      use:dndzone={{
        items: trackItems,
        flipDurationMs,
        dropTargetStyle: { outline: "none" },
        dragDisabled: sortBy !== "manual",
      }}
      onconsider={handleTrackDndConsider}
      onfinalize={handleTrackDndFinalize}
    >
      {#each trackItems as track, index (track.id)}
        {@const isMissing = !track.path}
        {@const isCurrentTrack = $currentFile === track.path}
        {@const isPlayingTrack = isCurrentTrack && $isPlaying && !$isPaused}
        <div
          class="group spotify-list-item flex items-center gap-3 py-2 mb-1 transition-all duration-200 {sortBy === 'manual' ? 'cursor-grab active:cursor-grabbing' : ''} {isCurrentTrack ? 'bg-white/10 ring-1 ring-white/5' : 'hover:bg-white/5'} {isMissing ? 'opacity-50' : ''}"
          animate:flip={{ duration: isTrackDragging ? flipDurationMs : 0 }}
          oncontextmenu={(e) => !isMissing && handleContextMenu(e, track)}
        >
          <!-- Drag Handle -->
          {#if sortBy === "manual"}
            <div class="text-white/30 hover:text-white/60 transition-colors flex-shrink-0">
              <Icon icon="mdi:drag-vertical" class="w-5 h-5" />
            </div>
          {/if}

          <!-- Track Number / Play Button / Playing Indicator / Missing Icon -->
          <div class="w-8 flex items-center justify-center flex-shrink-0">
            {#if isMissing}
              <Icon icon="mdi:file-alert" class="w-5 h-5 text-red-400" title={$t("playlists.fileMissing")} />
            {:else if isPlayingTrack}
              <div class="flex items-end gap-0.5 h-4">
                <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-1" style="height: 60%;"></div>
                <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-2" style="height: 100%;"></div>
                <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-3" style="height: 80%;"></div>
              </div>
            {:else}
              <span class="text-sm text-white/40 {isCurrentTrack ? 'text-[#1db954] font-semibold' : ''} group-hover:hidden">{index + 1}</span>
              <button
                class="hidden group-hover:flex items-center justify-center w-7 h-7 rounded-full bg-[#1db954] hover:scale-110 transition-transform shadow-lg"
                onclick={() => handlePlayTrack(track)}
                title="Play"
              >
                <Icon icon="mdi:play" class="w-4 h-4 text-black" />
              </button>
            {/if}
          </div>

          <!-- Track Info -->
          <div
            class="flex-1 min-w-0 {isMissing ? 'cursor-default' : 'cursor-pointer'}"
            onclick={() => !isMissing && handlePlayTrack(track)}
          >
            <p class="text-sm font-medium truncate transition-colors {isMissing ? 'text-red-400 line-through' : isCurrentTrack ? 'text-[#1db954]' : 'text-white group-hover:text-[#1db954]'}">
              {track.name}
            </p>
            <p class="text-xs text-white/40">
              {#if isMissing}
                <span class="text-red-400">{$t("playlists.fileMissing")}</span> •
                <button class="text-red-400 hover:text-red-300 underline" onclick={() => handleRemoveTrack(track.hash)}>{$t("playlists.remove")}</button>
              {:else}
                {track.bpm || 120} BPM • {#if (track.note_density || 0) < 3}{$t("library.easy")}{:else if (track.note_density || 0) < 6}{$t("library.medium")}{:else if (track.note_density || 0) < 10}{$t("library.hard")}{:else}{$t("library.expert")}{/if}
              {/if}
            </p>
          </div>

          <!-- Duration -->
          <div class="text-sm text-white/40 flex-shrink-0 tabular-nums">
            {track.duration
              ? `${Math.floor(track.duration / 60)}:${String(Math.floor(track.duration % 60)).padStart(2, "0")}`
              : "--:--"}
          </div>

          <!-- Remove Button -->
          <button
            class="p-1.5 rounded-full text-white/30 opacity-0 group-hover:opacity-100 hover:text-red-400 hover:bg-red-500/20 transition-all flex-shrink-0"
            onclick={(e) => {
              e.stopPropagation();
              handleRemoveTrack(track.hash);
            }}
            title={$t("playlists.removeFromPlaylist")}
          >
            <Icon icon="mdi:close" class="w-4 h-4" />
          </button>
        </div>
      {/each}
    </div>

    {#if trackItems.length === 0 && selectedPlaylist.tracks.length > 0 && searchQuery}
      <div class="flex-1 flex flex-col items-center justify-center text-white/40 py-12">
        <Icon icon="mdi:magnify" class="w-10 h-10 opacity-50 mb-4" />
        <p class="text-sm">{$t("common.noResults", { values: { query: searchQuery } })}</p>
      </div>
    {:else if selectedPlaylist.tracks.length === 0}
      <div class="flex-1 flex flex-col items-center justify-center text-white/40 py-12">
        <Icon
          icon="mdi:music-note-plus"
          class="w-12 h-12 mb-4 opacity-50"
        />
        <p class="text-sm">{$t("playlists.emptyPlaylist")}</p>
        <p class="text-xs mt-1">{$t("playlists.addSongsFromLibrary")}</p>
      </div>
    {/if}

    {#if selectedPlaylist.tracks.length > 1 && sortBy === "manual"}
      <div
        class="pt-4 mt-4 border-t border-white/10 flex items-center justify-center gap-2 text-white/30"
      >
        <Icon icon="mdi:gesture-swipe-vertical" class="w-4 h-4" />
        <p class="text-xs">{$t("playlists.dragToReorder")}</p>
      </div>
    {/if}
  {:else}
    <!-- Playlists List View -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h2 class="text-2xl font-bold">{$t("playlists.title")}</h2>
          <p class="text-sm text-white/60">
            {$savedPlaylists.length} {$t("playlists.title").toLowerCase()}
          </p>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="spotify-button spotify-button--secondary flex items-center gap-2"
            onclick={importPlaylist}
            disabled={isImporting}
            title={$t("playlists.import")}
          >
            <Icon icon={isImporting ? "mdi:loading" : "mdi:import"} class="w-4 h-4 {isImporting ? 'animate-spin' : ''}" />
            {$t("playlists.import")}
          </button>
          <button
            class="spotify-button spotify-button--secondary flex items-center gap-2"
            onclick={() => (showCreateModal = true)}
          >
            <Icon icon="mdi:plus" class="w-4 h-4" />
            {$t("playlists.new")}
          </button>
        </div>
      </div>
    </div>

    <!-- Playlists Grid with Drag and Drop -->
    {#if $savedPlaylists.length > 0}
      <div
        bind:this={scrollContainer}
        onscroll={handleScroll}
        class="flex-1 overflow-y-auto {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
        use:dndzone={{
          items,
          flipDurationMs,
          dropTargetStyle: { outline: "none" },
        }}
        onconsider={handleDndConsider}
        onfinalize={handleDndFinalize}
      >
        {#each items as playlist (playlist.id)}
          <div
            class="group spotify-card mb-2 cursor-grab active:cursor-grabbing"
            animate:flip={{ duration: isPlaylistDragging ? flipDurationMs : 0 }}
          >
            <div class="flex items-center gap-4">
              <!-- Drag Handle -->
              <div class="text-white/30 hover:text-white/60 transition-colors">
                <Icon icon="mdi:drag-vertical" class="w-5 h-5" />
              </div>

              <!-- Playlist Icon -->
              <div
                class="w-12 h-12 rounded bg-white/10 flex items-center justify-center flex-shrink-0"
              >
                <Icon icon="mdi:playlist-music" class="w-6 h-6 text-[#1db954]" />
              </div>

              <!-- Playlist Info -->
              <div
                class="flex-1 min-w-0 cursor-pointer"
                onclick={() => (selectedPlaylistId = playlist.id)}
              >
                {#if editingPlaylistId === playlist.id}
                  <input
                    type="text"
                    bind:value={editingName}
                    class="bg-white/10 border border-white/20 rounded px-2 py-1 text-sm font-semibold w-full"
                    onblur={saveEdit}
                    onkeydown={(e) => e.key === "Enter" && saveEdit()}
                    onclick={(e) => e.stopPropagation()}
                    autofocus
                  />
                {:else}
                  <p class="font-semibold text-white truncate hover:text-[#1db954] transition-colors">
                    {playlist.name}
                  </p>
                {/if}
                <p class="text-xs text-white/50">
                  {playlist.tracks.length} {$t("library.songs")}
                </p>
              </div>

              <!-- Actions -->
              <div
                class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <button
                  class="p-2 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-all"
                  onclick={(e) => {
                    e.stopPropagation();
                    handleLoadToQueue(playlist);
                  }}
                  title={$t("playlists.play")}
                >
                  <Icon icon="mdi:play" class="w-5 h-5" />
                </button>
                <button
                  class="p-2 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-all"
                  onclick={(e) => {
                    e.stopPropagation();
                    exportPlaylist(playlist);
                  }}
                  disabled={playlist.tracks.length === 0 || isExporting}
                  title={$t("playlists.exportPlaylist")}
                >
                  <Icon icon={isExporting ? "mdi:loading" : "mdi:export"} class="w-4 h-4 {isExporting ? 'animate-spin' : ''}" />
                </button>
                <button
                  class="p-2 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-all"
                  onclick={(e) => {
                    e.stopPropagation();
                    startEditing(playlist);
                  }}
                  title={$t("playlists.rename")}
                >
                  <Icon icon="mdi:pencil" class="w-4 h-4" />
                </button>
                <button
                  class="p-2 rounded-full hover:bg-red-500/20 text-white/60 hover:text-red-400 transition-all"
                  onclick={(e) => {
                    e.stopPropagation();
                    handleDelete(playlist.id);
                  }}
                  title={$t("playlists.delete")}
                >
                  <Icon icon="mdi:delete-outline" class="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>

      <div
        class="pt-4 mt-4 border-t border-white/10 flex items-center justify-center gap-2 text-white/30"
      >
        <Icon icon="mdi:gesture-swipe-vertical" class="w-4 h-4" />
        <p class="text-xs">{$t("playlists.dragToReorder")}</p>
      </div>
    {:else}
      <div
        class="flex-1 flex flex-col items-center justify-center text-white/40 py-16"
        transition:fade
      >
        <div
          class="w-20 h-20 rounded-full bg-white/5 flex items-center justify-center mb-6"
        >
          <Icon icon="mdi:playlist-plus" class="w-10 h-10 opacity-50" />
        </div>
        <p class="text-lg font-semibold mb-2 text-white/60">{$t("playlists.noPlaylists")}</p>
        <p class="text-sm mb-4">{$t("playlists.createToOrganize")}</p>
        <button
          class="spotify-button spotify-button--primary flex items-center gap-2"
          onclick={() => (showCreateModal = true)}
        >
          <Icon icon="mdi:plus" class="w-4 h-4" />
          {$t("playlists.createPlaylist")}
        </button>
      </div>
    {/if}
  {/if}
</div>

<!-- Create Playlist Modal -->
{#if showCreateModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <!-- Backdrop -->
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { showCreateModal = false; newPlaylistName = ""; }}
    ></button>

    <!-- Modal -->
    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[400px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-white/10">
        <h3 class="text-lg font-bold">{$t("playlists.createPlaylist")}</h3>
        <button
          class="p-1 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors"
          onclick={() => { showCreateModal = false; newPlaylistName = ""; }}
        >
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-4 space-y-4">
        <!-- Name Input -->
        <div>
          <label class="block text-sm font-medium text-white/70 mb-2">{$t("playlists.playlistName")}</label>
          <input
            type="text"
            placeholder={$t("playlists.myPlaylist")}
            bind:value={newPlaylistName}
            class="w-full px-4 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent transition-all"
            onkeydown={(e) => e.key === "Enter" && handleCreate()}
            autofocus
          />
        </div>

        <!-- Action Buttons -->
        <div class="flex gap-2 justify-end pt-2">
          <button
            class="px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
            onclick={() => { showCreateModal = false; newPlaylistName = ""; }}
          >
            {$t("common.cancel")}
          </button>
          <button
            class="px-4 py-2.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-black font-medium text-sm transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={handleCreate}
            disabled={!newPlaylistName.trim()}
          >
            {$t("common.create")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Modal -->
{#if deleteConfirmId}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <!-- Backdrop -->
    <button
      class="absolute inset-0 bg-black/60"
      onclick={cancelDelete}
    ></button>

    <!-- Modal -->
    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[400px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-white/10">
        <h3 class="text-lg font-bold">{$t("playlists.deletePlaylist")}</h3>
        <button
          class="p-1 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors"
          onclick={cancelDelete}
        >
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-4 space-y-4">
        <!-- Warning Icon -->
        <div class="flex items-center gap-4">
          <div class="w-16 h-16 rounded-xl bg-red-500/10 flex items-center justify-center">
            <Icon icon="mdi:delete-alert" class="w-8 h-8 text-red-400" />
          </div>
          <div class="text-left">
            <p class="font-semibold text-white">{$savedPlaylists.find(p => p.id === deleteConfirmId)?.name || 'Playlist'}</p>
            <p class="text-sm text-white/50">{$t("playlists.cannotBeUndone")}</p>
          </div>
        </div>

        <!-- Action Buttons -->
        <div class="flex gap-2 justify-end pt-2">
          <button
            class="px-4 py-2.5 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium text-sm transition-colors"
            onclick={cancelDelete}
          >
            {$t("common.cancel")}
          </button>
          <button
            class="px-4 py-2.5 rounded-lg bg-red-500 hover:bg-red-600 text-white font-medium text-sm transition-colors"
            onclick={confirmDelete}
          >
            {$t("common.delete")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<svelte:window onclick={() => contextMenu = null} />

<SongContextMenu {contextMenu} onClose={() => contextMenu = null} />

