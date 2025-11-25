<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
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
  } from "../stores/player.js";

  let searchQuery = "";
  let showPlaylistMenu = null;
  let toast = null;
  let toastTimeout = null;
  let isDragOver = false;
  let isImporting = false;
  let unlistenDrop = null;
  let unlistenHover = null;
  let unlistenCancel = null;

  onMount(async () => {
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

  function showToast(message, type = "success") {
    if (toastTimeout) clearTimeout(toastTimeout);
    toast = { message, type };
    toastTimeout = setTimeout(() => {
      toast = null;
    }, 2000);
  }

  async function handlePlay(file) {
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
    const wasFavorite = $favorites.some((f) => f.path === file.path);
    toggleFavorite(file);
    showToast(
      wasFavorite ? "Removed from favorites" : "Added to favorites",
      wasFavorite ? "info" : "success"
    );
  }

  // Reactive favorite lookup using a Set for O(1) performance
  $: favoritePaths = new Set($favorites.map(f => f.path));

  $: filteredFiles = $midiFiles.filter((file) =>
    file.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

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

  <!-- Header -->
  <div class="mb-4">
    <h2 class="text-2xl font-bold mb-2">Your Library</h2>
    <p class="text-sm text-white/60 mb-4">
      {filteredFiles.length} of {$midiFiles.length} songs
    </p>

    <!-- Search Input -->
    <div class="relative">
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
  </div>

  <!-- Song List (Scrollable) -->
  <div class="flex-1 overflow-y-auto space-y-1 pr-2">
    {#each filteredFiles as file, index (file.path)}
      <div
        class="group spotify-list-item flex items-center gap-4 py-2 transition-all duration-200 {$currentFile ===
        file.path
          ? 'bg-white/10 ring-1 ring-white/5'
          : 'hover:bg-white/5'}"
        in:fly={{ y: 10, duration: 200, delay: Math.min(index * 20, 200) }}
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
            <button
              class="hidden group-hover:flex items-center justify-center w-7 h-7 rounded-full bg-[#1db954] hover:scale-110 transition-transform shadow-lg"
              onclick={() => handlePlay(file)}
              title="Play"
            >
              <Icon icon="mdi:play" class="w-4 h-4 text-black" />
            </button>
          {/if}
        </div>

        <!-- Song Info -->
        <div
          class="flex-1 min-w-0"
          role="button"
          tabindex="0"
          onclick={() => handlePlay(file)}
          onkeydown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              handlePlay(file);
            }
          }}
        >
          <p
            class="text-sm font-medium text-white truncate transition-colors {$currentFile ===
            file.path
              ? 'text-[#1db954]'
              : 'group-hover:text-white'}"
          >
            {file.name}
          </p>
          <p class="text-xs text-white/40">MIDI Track</p>
        </div>

        <!-- Duration -->
        <div class="text-sm text-white/40 flex-shrink-0 tabular-nums">
          {file.duration
            ? `${Math.floor(file.duration / 60)}:${String(Math.floor(file.duration % 60)).padStart(2, "0")}`
            : "--:--"}
        </div>

        <!-- Action Buttons -->
        <div class="flex items-center gap-1 flex-shrink-0">
          <!-- Favorite Button -->
          <button
            class="p-1.5 rounded-full transition-all {favoritePaths.has(file.path)
              ? 'text-[#1db954]'
              : 'text-white/30 opacity-0 group-hover:opacity-100 hover:text-white'}"
            onclick={(e) => {
              e.stopPropagation();
              handleToggleFavorite(file);
            }}
            title={favoritePaths.has(file.path)
              ? "Remove from favorites"
              : "Add to favorites"}
          >
            <Icon
              icon={favoritePaths.has(file.path) ? "mdi:heart" : "mdi:heart-outline"}
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
        </div>
      </div>
    {/each}
  </div>

  {#if filteredFiles.length === 0 && searchQuery}
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
  {:else if $midiFiles.length === 0}
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

<svelte:window
  onclick={() => {
    showPlaylistMenu = null;
  }}
/>
