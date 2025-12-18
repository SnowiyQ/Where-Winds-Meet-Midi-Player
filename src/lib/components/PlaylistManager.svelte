<script>
  import Icon from "@iconify/svelte";
  import { flip } from "svelte/animate";
  import { dndzone } from "svelte-dnd-action";
  import { t } from "svelte-i18n";
  import {
    playlist,
    currentFile,
    currentIndex,
    playMidi,
    isPlaying,
    isPaused,
  } from "../stores/player.js";
  import SongContextMenu from "./SongContextMenu.svelte";
  import SearchSort from "./SearchSort.svelte";

  // Search & Sort
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

  function handleContextMenu(e, file) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, file };
  }

  const flipDurationMs = 300;

  // Filter and sort queue
  $: filteredPlaylist = (() => {
    let result = searchQuery.trim()
      ? $playlist.filter(f => f.name.toLowerCase().includes(searchQuery.toLowerCase()))
      : [...$playlist];

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

  // Use stable IDs based on hash + index (allows duplicates in queue)
  let items = [];
  $: {
    // Only update items if playlist changed (not during drag)
    if (!isDragging) {
      items = filteredPlaylist.map((file, index) => ({
        ...file,
        id: `${file.hash}-${index}`,
        originalIndex: $playlist.indexOf(file),
      }));
    }
  }

  let isDragging = false;

  function handleDndConsider(e) {
    isDragging = true;
    items = e.detail.items;
  }

  function handleDndFinalize(e) {
    items = e.detail.items;
    // Update the playlist store
    playlist.set(items.map(({ id, ...file }) => file));
    isDragging = false;
  }

  function removeFromPlaylist(index) {
    playlist.update((list) => {
      const newList = [...list];
      newList.splice(index, 1);

      // Update currentIndex if needed
      const $currentIndex = $currentIndex;
      if (index < $currentIndex) {
        currentIndex.set($currentIndex - 1);
      } else if (index === $currentIndex && index >= newList.length) {
        currentIndex.set(Math.max(0, newList.length - 1));
      }

      return newList;
    });
  }

  function clearPlaylist() {
    playlist.set([]);
    currentIndex.set(0);
  }

  async function playFromPlaylist(index) {
    currentIndex.set(index);
    await playMidi($playlist[index].path);
  }
</script>

<div class="h-full flex flex-col" role="region" aria-label="Playlist manager">
  <!-- Header -->
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-2xl font-bold mb-2">{$t("queue.title")}</h2>
      <p class="text-sm text-white/60">
        {$playlist.length} {$t("library.songs")}
        {#if $playlist.length > 0}
          <span class="text-[#1db954]">
            • {$t("queue.nowPlaying")} {$currentIndex + 1} / {$playlist.length}
          </span>
        {/if}
      </p>
    </div>
    {#if $playlist.length > 0}
      <button
        class="spotify-button spotify-button--secondary text-xs flex items-center gap-2"
        onclick={clearPlaylist}
      >
        <Icon icon="mdi:playlist-remove" class="w-4 h-4" />
        {$t("favorites.clearAll")}
      </button>
    {/if}
  </div>

  <!-- Search + Sort -->
  {#if $playlist.length > 0}
    <div class="mb-4">
      <SearchSort
        bind:searchQuery
        bind:sortBy
        placeholder={$t("queue.title") + "..."}
        {sortOptions}
      />
    </div>
  {/if}

  <!-- Playlist Items with DnD -->
  {#if filteredPlaylist.length > 0}
    <div
      class="flex-1 overflow-y-auto space-y-1 dnd-zone"
      role="list"
      aria-live="polite"
      use:dndzone={{
        items,
        flipDurationMs,
        dropTargetStyle: { outline: "none" },
        dragDisabled: sortBy !== "manual",
      }}
      onconsider={handleDndConsider}
      onfinalize={handleDndFinalize}
    >
      {#each items as item, index (item.id)}
        <div
          class="group spotify-list-item flex items-center gap-4 py-2 transition-all duration-200 {sortBy === 'manual' ? 'cursor-grab active:cursor-grabbing' : ''} {$currentFile ===
          item.path
            ? 'bg-white/10 ring-1 ring-white/5'
            : 'hover:bg-white/5'}"
          role="listitem"
          animate:flip={{ duration: isDragging ? flipDurationMs : 0 }}
          oncontextmenu={(e) => handleContextMenu(e, item)}
        >
          <!-- Drag Handle -->
          {#if sortBy === "manual"}
            <div
              class="w-6 flex items-center justify-center text-white/30 hover:text-white/60 flex-shrink-0 transition-colors"
            >
              <Icon icon="mdi:drag-vertical" class="w-5 h-5" />
            </div>
          {/if}

          <!-- Number / Play Button / Playing Indicator -->
          <div class="w-8 flex items-center justify-center flex-shrink-0">
            {#if $currentFile === item.path && $isPlaying && !$isPaused}
              <!-- Playing indicator (animated bars) -->
              <div class="flex items-end gap-0.5 h-4">
                <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-1" style="height: 60%;"></div>
                <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-2" style="height: 100%;"></div>
                <div class="w-0.5 bg-[#1db954] rounded-full animate-music-bar-3" style="height: 80%;"></div>
              </div>
            {:else}
              <span
                class="text-sm text-white/40 {$currentFile === item.path
                  ? 'text-[#1db954] font-semibold'
                  : ''} group-hover:hidden">{index + 1}</span
              >
              <button
                class="hidden group-hover:flex items-center justify-center w-6 h-6 rounded-full bg-[#1db954] hover:scale-110 transition-transform"
                onclick={() => playFromPlaylist(index)}
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
            onclick={() => playFromPlaylist(index)}
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                playFromPlaylist(index);
              }
            }}
          >
            <p
              class="text-sm font-medium text-white truncate transition-colors {$currentFile ===
              item.path
                ? 'text-[#1db954]'
                : 'group-hover:text-white'}"
            >
              {item.name}
            </p>
            <p class="text-xs text-white/40">
              {item.bpm || 120} BPM • {#if (item.note_density || 0) < 3}{$t("library.easy")}{:else if (item.note_density || 0) < 6}{$t("library.medium")}{:else if (item.note_density || 0) < 10}{$t("library.hard")}{:else}{$t("library.expert")}{/if}
            </p>
          </div>

          <!-- Duration -->
          <div class="text-sm text-white/40 flex-shrink-0 tabular-nums">
            {item.duration
              ? `${Math.floor(item.duration / 60)}:${String(Math.floor(item.duration % 60)).padStart(2, "0")}`
              : "--:--"}
          </div>

          <!-- Remove Button -->
          <button
            class="opacity-0 group-hover:opacity-100 text-white/40 hover:text-red-400 transition-all flex-shrink-0 p-1 rounded hover:bg-red-400/10"
            onclick={(e) => {
              e.stopPropagation();
              removeFromPlaylist(index);
            }}
            title={$t("playlists.remove")}
          >
            <Icon icon="mdi:close" class="w-4 h-4" />
          </button>
        </div>
      {/each}
    </div>

    {#if sortBy === "manual"}
      <div
        class="pt-4 mt-4 border-t border-white/10 flex items-center justify-center gap-2 text-white/30"
      >
        <Icon icon="mdi:gesture-swipe-vertical" class="w-4 h-4" />
        <p class="text-xs">{$t("playlists.dragToReorder")}</p>
      </div>
    {/if}
  {:else if $playlist.length > 0 && searchQuery}
    <div class="flex-1 flex flex-col items-center justify-center text-white/40 py-16">
      <Icon icon="mdi:magnify" class="w-10 h-10 opacity-50 mb-4" />
      <p class="text-sm">{$t("common.noResults", { values: { query: searchQuery } })}</p>
    </div>
  {:else}
    <div
      class="flex-1 flex flex-col items-center justify-center text-white/40 py-16"
    >
      <div
        class="w-20 h-20 rounded-full bg-white/5 flex items-center justify-center mb-6"
      >
        <Icon icon="mdi:playlist-music" class="w-10 h-10 opacity-50" />
      </div>
      <p class="text-lg font-semibold mb-2 text-white/60">{$t("queue.emptyQueue")}</p>
      <p class="text-sm text-white/40">{$t("queue.addFromLibrary")}</p>
    </div>
  {/if}
</div>

<svelte:window onclick={() => contextMenu = null} />

<SongContextMenu {contextMenu} onClose={() => contextMenu = null} />

<style>
  .dnd-zone {
    min-height: 100px;
  }

  :global(.dnd-zone > div) {
    outline: none !important;
  }
</style>
