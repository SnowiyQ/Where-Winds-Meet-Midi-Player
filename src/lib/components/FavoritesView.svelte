<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { dndzone } from "svelte-dnd-action";
  import { onMount } from "svelte";
  import {
    favorites,
    currentFile,
    playMidi,
    playlist,
    isPlaying,
    isPaused,
    toggleFavorite,
  } from "../stores/player.js";

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
        id: file.path,
      }));
    }
  }

  function handleDndConsider(e) {
    isDragging = true;
    items = e.detail.items;
  }

  function handleDndFinalize(e) {
    items = e.detail.items;
    favorites.set(items.map(({ id, ...file }) => file));
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
    playlist.update((list) => {
      if (!list.find((f) => f.path === file.path)) {
        return [...list, file];
      }
      return list;
    });
  }

  function playAllFavorites() {
    if ($favorites.length === 0) return;
    playlist.set([...$favorites]);
    playMidi($favorites[0].path);
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

    {#if $favorites.length > 0}
      <button
        class="spotify-button spotify-button--primary flex items-center gap-2"
        onclick={playAllFavorites}
      >
        <Icon icon="mdi:play" class="w-5 h-5" />
        Play All
      </button>
    {/if}
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
        <div
          class="group spotify-list-item flex items-center gap-4 py-2 cursor-grab active:cursor-grabbing transition-all duration-200 {$currentFile ===
          item.path
            ? 'bg-white/10 ring-1 ring-white/5'
            : 'hover:bg-white/5'}"
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
                onclick={() => handlePlay(item)}
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
            onclick={() => handlePlay(item)}
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                handlePlay(item);
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

<style>
  .dnd-zone {
    min-height: 100px;
  }

  :global(.dnd-zone > div) {
    outline: none !important;
  }
</style>
