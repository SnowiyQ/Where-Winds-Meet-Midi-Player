<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { dndzone } from "svelte-dnd-action";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import {
    favorites,
    midiFiles,
    loadMidiFiles,
    currentFile,
    playMidi,
    playlist,
    isPlaying,
    isPaused,
    toggleFavorite,
    clearAllFavorites,
    reorderFavorites,
    missingFiles,
    removeDeletedFile,
  } from "../stores/player.js";
  import SongContextMenu from "./SongContextMenu.svelte";

  let showClearModal = false;
  let contextMenu = null;
  let isExporting = false;
  let isImporting = false;
  let toast = null;
  let toastTimeout = null;

  function showToast(message, type = "success") {
    if (toastTimeout) clearTimeout(toastTimeout);
    toast = { message, type };
    toastTimeout = setTimeout(() => { toast = null; }, 3000);
  }

  function handleRemoveMissing(file) {
    removeDeletedFile(file.hash);
    missingFiles.update(set => {
      const newSet = new Set(set);
      newSet.delete(file.hash);
      return newSet;
    });
    showToast(`Removed "${file.name}"`, "success");
  }

  function handleContextMenu(e, file) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, file };
  }

  const flipDurationMs = 300;

  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

  // Drag and drop state
  let items = [];
  let isDragging = false;

  $: {
    if (!isDragging) {
      items = $favorites.map((file) => ({
        ...file,
        id: file.hash,
      }));
    }
  }

  function handleDndConsider(e) {
    isDragging = true;
    items = e.detail.items;
  }

  function handleDndFinalize(e) {
    items = e.detail.items;
    reorderFavorites(items.map(({ id, ...file }) => file));
    isDragging = false;
  }

  function handleScroll(e) {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    showTopMask = scrollTop > 10;
    showBottomMask = scrollTop + clientHeight < scrollHeight - 10;
  }

  onMount(() => {
    setTimeout(() => {
      if (scrollContainer) {
        const { scrollHeight, clientHeight } = scrollContainer;
        showBottomMask = scrollHeight > clientHeight;
      }
    }, 100);
  });

  async function playNow(file) {
    try {
      // Add to playlist if not already there
      playlist.update((list) => {
        if (!list.find((f) => f.path === file.path)) {
          return [...list, file];
        }
        return list;
      });
      await playMidi(file.path);
    } catch (error) {
      if (error.message === 'FILE_MISSING') {
        missingFiles.update(set => {
          const newSet = new Set(set);
          newSet.add(file.hash);
          return newSet;
        });
        showToast(`"${file.name}" no longer exists`, "error");
      } else {
        showToast(error.toString(), "error");
      }
    }
  }

  function addToQueue(file) {
    playlist.update((list) => {
      if (!list.find((f) => f.path === file.path)) {
        return [...list, file];
      }
      return list;
    });
  }

  async function playAllFavorites() {
    if ($favorites.length === 0) return;
    // Filter out missing files
    const validFavorites = $favorites.filter(f => !$missingFiles.has(f.hash));
    if (validFavorites.length === 0) {
      showToast("All favorites are missing", "error");
      return;
    }
    playlist.set([...validFavorites]);
    try {
      await playMidi(validFavorites[0].path);
    } catch (error) {
      if (error.message === 'FILE_MISSING') {
        missingFiles.update(set => {
          const newSet = new Set(set);
          newSet.add(validFavorites[0].hash);
          return newSet;
        });
        showToast(`"${validFavorites[0].name}" no longer exists`, "error");
      }
    }
  }

  async function exportFavorites() {
    if ($favorites.length === 0 || isExporting) return;

    try {
      isExporting = true;
      const exportPath = await save({
        title: "Export Favorites",
        defaultPath: "favorites.zip",
        filters: [{ name: "Zip Archive", extensions: ["zip"] }],
      });

      if (exportPath) {
        // Hydrate favorites with paths from midiFiles using hash
        const filesByHash = new Map($midiFiles.map(f => [f.hash, f]));
        const hydratedFavorites = $favorites
          .map(fav => filesByHash.get(fav.hash) || fav)
          .filter(fav => fav.path); // Only include files with valid paths

        await invoke("export_favorites", {
          favorites: hydratedFavorites,
          exportPath,
        });
      }
    } catch (error) {
      console.error("Failed to export favorites:", error);
    } finally {
      isExporting = false;
    }
  }

  async function importFavorites() {
    if (isImporting) return;

    try {
      isImporting = true;
      const zipPath = await open({
        title: "Import Favorites",
        filters: [{ name: "Zip Archive", extensions: ["zip"] }],
      });

      if (zipPath) {
        const result = await invoke("import_zip", { zipPath });

        // Reload library to include new files
        await loadMidiFiles();

        // Add imported files to favorites if it was a favorites export
        if (result.export_type === "favorites") {
          for (const file of result.imported_files) {
            // Check if not already in favorites
            if (!$favorites.find(f => f.hash === file.hash)) {
              toggleFavorite(file);
            }
          }
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
  <!-- Header -->
  <div class="mb-6">
    <div class="flex items-center gap-4 mb-4">
      <div
        class="w-16 h-16 rounded-lg bg-white/10 flex items-center justify-center"
      >
        <Icon icon="mdi:heart" class="w-8 h-8 text-[#1db954]" />
      </div>
      <div>
        <h2 class="text-2xl font-bold">Favorites</h2>
        <p class="text-sm text-white/60">{$favorites.length} liked songs</p>
      </div>
    </div>

    <div class="flex items-center gap-2">
      {#if $favorites.length > 0}
        <button
          class="spotify-button spotify-button--primary flex items-center gap-2"
          onclick={playAllFavorites}
        >
          <Icon icon="mdi:play" class="w-5 h-5" />
          Play All
        </button>
        <button
          class="px-3 py-2 rounded-full bg-white/10 hover:bg-white/20 text-white/60 hover:text-white text-sm font-medium transition-all flex items-center gap-1.5"
          onclick={exportFavorites}
          disabled={isExporting}
          title="Export favorites with MIDI files"
        >
          <Icon icon={isExporting ? "mdi:loading" : "mdi:export"} class="w-4 h-4 {isExporting ? 'animate-spin' : ''}" />
          Export
        </button>
      {/if}
      <button
        class="px-3 py-2 rounded-full bg-white/10 hover:bg-white/20 text-white/60 hover:text-white text-sm font-medium transition-all flex items-center gap-1.5"
        onclick={importFavorites}
        disabled={isImporting}
        title="Import favorites from zip"
      >
        <Icon icon={isImporting ? "mdi:loading" : "mdi:import"} class="w-4 h-4 {isImporting ? 'animate-spin' : ''}" />
        Import
      </button>
      {#if $favorites.length > 0}
        <button
          class="px-3 py-2 rounded-full bg-white/10 hover:bg-red-500/20 text-white/60 hover:text-red-400 text-sm font-medium transition-all flex items-center gap-1.5"
          onclick={() => showClearModal = true}
          title="Clear all favorites"
        >
          <Icon icon="mdi:delete-sweep" class="w-4 h-4" />
          Clear All
        </button>
      {/if}
    </div>
  </div>

  <!-- Favorites List with DnD -->
  {#if $favorites.length > 0}
    <div
      bind:this={scrollContainer}
      onscroll={handleScroll}
      class="flex-1 overflow-y-auto space-y-1 pr-2 dnd-zone {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
      use:dndzone={{
        items,
        flipDurationMs,
        dropTargetStyle: { outline: "none" },
      }}
      onconsider={handleDndConsider}
      onfinalize={handleDndFinalize}
    >
      {#each items as item, index (item.id)}
        {@const isMissing = !item.path || $missingFiles.has(item.hash)}
        <div
          class="group spotify-list-item flex items-center gap-4 py-2 cursor-grab active:cursor-grabbing transition-all duration-200 {$currentFile ===
          item.path
            ? 'bg-white/10 ring-1 ring-white/5'
            : 'hover:bg-white/5'} {isMissing ? 'opacity-50' : ''}"
          animate:flip={{ duration: flipDurationMs }}
          oncontextmenu={(e) => handleContextMenu(e, item)}
        >
          <!-- Drag Handle -->
          <div
            class="w-6 flex items-center justify-center text-white/30 hover:text-white/60 flex-shrink-0 transition-colors"
          >
            <Icon icon="mdi:drag-vertical" class="w-5 h-5" />
          </div>

          <!-- Number / Play Button / Playing Indicator -->
          <div class="w-8 flex items-center justify-center flex-shrink-0">
            {#if isMissing}
              <Icon icon="mdi:file-alert" class="w-5 h-5 text-red-400" title="File missing" />
            {:else if $currentFile === item.path && $isPlaying && !$isPaused}
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
                class="text-sm text-white/40 {$currentFile === item.path
                  ? 'text-[#1db954] font-semibold'
                  : ''} group-hover:hidden">{index + 1}</span
              >
              <button
                class="hidden group-hover:flex items-center justify-center w-7 h-7 rounded-full bg-[#1db954] hover:scale-110 transition-transform shadow-lg"
                onclick={() => playNow(item)}
                title="Play"
              >
                <Icon icon="mdi:play" class="w-4 h-4 text-black" />
              </button>
            {/if}
          </div>

          <!-- Song Info -->
          <div
            class="flex-1 min-w-0 cursor-pointer"
            role="button"
            tabindex="0"
            onclick={() => isMissing ? null : playNow(item)}
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                if (!isMissing) playNow(item);
              }
            }}
          >
            <p
              class="text-sm font-medium truncate transition-colors {isMissing ? 'text-red-400 line-through' : $currentFile === item.path ? 'text-[#1db954]' : 'text-white group-hover:text-white'}"
            >
              {item.name}
            </p>
            <p class="text-xs text-white/40">
              {#if isMissing}
                <span class="text-red-400">File missing</span> •
                <button class="text-red-400 hover:text-red-300 underline" onclick={() => handleRemoveMissing(item)}>Remove</button>
              {:else}
                {item.bpm || 120} BPM • {#if (item.note_density || 0) < 3}Easy{:else if (item.note_density || 0) < 6}Medium{:else if (item.note_density || 0) < 10}Hard{:else}Expert{/if}
              {/if}
            </p>
          </div>

          <!-- Duration -->
          <div class="text-sm text-white/40 flex-shrink-0 tabular-nums">
            {item.duration
              ? `${Math.floor(item.duration / 60)}:${String(Math.floor(item.duration % 60)).padStart(2, "0")}`
              : "--:--"}
          </div>

          <!-- Action Buttons -->
          <div class="flex items-center gap-1 flex-shrink-0">
            <button
              class="p-1.5 rounded-full text-white/30 opacity-0 group-hover:opacity-100 hover:text-white transition-all"
              onclick={(e) => {
                e.stopPropagation();
                addToQueue(item);
              }}
              title="Add to queue"
            >
              <Icon icon="mdi:playlist-plus" class="w-5 h-5" />
            </button>

            <button
              class="p-1.5 rounded-full text-[#1db954] hover:text-red-400 transition-all"
              onclick={(e) => {
                e.stopPropagation();
                toggleFavorite(item);
              }}
              title="Remove from favorites"
            >
              <Icon icon="mdi:heart" class="w-5 h-5" />
            </button>
          </div>
        </div>
      {/each}
    </div>

    <div
      class="pt-4 mt-4 border-t border-white/10 flex items-center justify-center gap-2 text-white/30"
    >
      <Icon icon="mdi:gesture-swipe-vertical" class="w-4 h-4" />
      <p class="text-xs">Drag to reorder</p>
    </div>
  {/if}

  {#if $favorites.length === 0}
    <div
      class="flex-1 flex flex-col items-center justify-center text-white/40 py-16"
      transition:fade
    >
      <div
        class="w-20 h-20 rounded-full bg-white/5 flex items-center justify-center mb-6"
      >
        <Icon icon="mdi:heart-outline" class="w-10 h-10 opacity-50" />
      </div>
      <p class="text-lg font-semibold mb-2 text-white/60">No favorites yet</p>
      <p class="text-sm text-center">
        Click the heart icon on songs<br />to add them to favorites
      </p>
    </div>
  {/if}
</div>

<!-- Clear All Confirmation Modal -->
{#if showClearModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => showClearModal = false}
    ></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[360px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <div class="p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-red-500/20 flex items-center justify-center mx-auto mb-3">
          <Icon icon="mdi:heart-off" class="w-6 h-6 text-red-400" />
        </div>
        <h3 class="text-lg font-bold mb-2">Clear All Favorites?</h3>
        <p class="text-sm text-white/60 mb-1">This will remove all {$favorites.length} songs from your favorites.</p>
        <p class="text-xs text-white/40">This cannot be undone.</p>
      </div>

      <div class="flex gap-2 p-4 pt-0">
        <button
          class="flex-1 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition-colors"
          onclick={() => showClearModal = false}
        >
          Cancel
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-red-500 hover:bg-red-600 text-white font-medium text-sm transition-colors"
          onclick={() => { clearAllFavorites(); showClearModal = false; }}
        >
          Clear All
        </button>
      </div>
    </div>
  </div>
{/if}

<svelte:window onclick={() => contextMenu = null} />

<SongContextMenu {contextMenu} onClose={() => contextMenu = null} />

<!-- Toast -->
{#if toast}
  <div
    class="fixed bottom-20 left-1/2 -translate-x-1/2 px-4 py-2 rounded-lg shadow-lg z-50 {toast.type === 'error' ? 'bg-red-500' : 'bg-[#1db954]'} text-white text-sm font-medium"
    transition:fly={{ y: 20, duration: 200 }}
  >
    {toast.message}
  </div>
{/if}

<style>
  .dnd-zone {
    min-height: 100px;
  }

  :global(.dnd-zone > div) {
    outline: none !important;
  }
</style>
