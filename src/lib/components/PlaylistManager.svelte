<script>
  import Icon from "@iconify/svelte";
  import { flip } from "svelte/animate";
  import { dndzone } from "svelte-dnd-action";
  import {
    playlist,
    currentFile,
    currentIndex,
    playMidi,
    isPlaying,
    isPaused,
  } from "../stores/player.js";

  const flipDurationMs = 300;

  // Use stable IDs based on path only
  let items = [];
  $: {
    // Only update items if playlist changed (not during drag)
    if (!isDragging) {
      items = $playlist.map((file) => ({
        ...file,
        id: file.path,
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
      <h2 class="text-2xl font-bold mb-2">Queue</h2>
      <p class="text-sm text-white/60">
        {$playlist.length} songs
        {#if $playlist.length > 0}
          <span class="text-[#1db954]">
            • Playing {$currentIndex + 1} of {$playlist.length}
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
        Clear
      </button>
    {/if}
  </div>

  <!-- Playlist Items with DnD -->
  {#if $playlist.length > 0}
    <div
      class="flex-1 overflow-y-auto space-y-1 dnd-zone"
      role="list"
      aria-live="polite"
      use:dndzone={{
        items,
        flipDurationMs,
        dropTargetStyle: { outline: "none" },
      }}
      onconsider={handleDndConsider}
      onfinalize={handleDndFinalize}
    >
      {#each items as item, index (item.id)}
        <div
          class="group spotify-list-item flex items-center gap-4 py-2 cursor-grab active:cursor-grabbing transition-all duration-200 {$currentFile ===
          item.path
            ? 'bg-white/10 ring-1 ring-white/5'
            : 'hover:bg-white/5'}"
          role="listitem"
          animate:flip={{ duration: flipDurationMs }}
        >
          <!-- Drag Handle -->
          <div
            class="w-6 flex items-center justify-center text-white/30 hover:text-white/60 flex-shrink-0 transition-colors"
          >
            <Icon icon="mdi:drag-vertical" class="w-5 h-5" />
          </div>

          <!-- Number / Play Button / Playing Indicator -->
          <div class="w-8 flex items-center justify-center flex-shrink-0">
            {#if $currentFile === item.path && $isPlaying && !$isPaused}
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
            <p class="text-xs text-white/40">MIDI Track</p>
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
            title="Remove from queue"
          >
            <Icon icon="mdi:close" class="w-4 h-4" />
          </button>
        </div>
      {/each}
    </div>

    <div
      class="pt-4 mt-4 border-t border-white/10 flex items-center justify-center gap-2 text-white/30"
    >
      <Icon icon="mdi:gesture-swipe-vertical" class="w-4 h-4" />
      <p class="text-xs">Drag to reorder</p>
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
      <p class="text-lg font-semibold mb-2 text-white/60">Queue is empty</p>
      <p class="text-sm text-white/40">Add tracks from your library</p>
    </div>
  {/if}
</div>

<style>
  .dnd-zone {
    min-height: 100px;
  }

  :global(.dnd-zone > div) {
    outline: none !important;
  }
</style>
