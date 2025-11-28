<script>
  import Icon from "@iconify/svelte";
  import { onMount } from "svelte";
  import { midiFiles, favorites, savedPlaylists, stats } from "../stores/player.js";

  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

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

  function formatTime(seconds) {
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    if (hrs > 0) {
      return `${hrs}h ${mins}m`;
    }
    return `${mins}m`;
  }

  function formatDate(isoString) {
    if (!isoString) return "Never";
    const date = new Date(isoString);
    return date.toLocaleDateString();
  }

  // Get top 5 most played songs
  $: topSongs = Object.entries($stats.mostPlayed || {})
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5);

  $: librarySize = $midiFiles.length;
  $: favoritesSize = $favorites.length;
  $: playlistsSize = $savedPlaylists.length;
</script>

<div
  bind:this={scrollContainer}
  onscroll={handleScroll}
  class="h-full overflow-y-auto {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
>
  <h2 class="text-2xl font-bold mb-6">Statistics</h2>

  <!-- Overview Cards -->
  <div class="grid grid-cols-2 gap-3 mb-6">
    <div class="bg-white/5 rounded-xl p-4">
      <div class="flex items-center gap-2 mb-2">
        <Icon icon="mdi:music-note" class="w-5 h-5 text-[#1db954]" />
        <span class="text-sm text-white/60">Songs Played</span>
      </div>
      <p class="text-2xl font-bold">{$stats.songsPlayed}</p>
    </div>

    <div class="bg-white/5 rounded-xl p-4">
      <div class="flex items-center gap-2 mb-2">
        <Icon icon="mdi:counter" class="w-5 h-5 text-[#1db954]" />
        <span class="text-sm text-white/60">Sessions</span>
      </div>
      <p class="text-2xl font-bold">{$stats.sessionsCount}</p>
    </div>

    <div class="bg-white/5 rounded-xl p-4">
      <div class="flex items-center gap-2 mb-2">
        <Icon icon="mdi:calendar" class="w-5 h-5 text-[#1db954]" />
        <span class="text-sm text-white/60">First Used</span>
      </div>
      <p class="text-lg font-bold">{formatDate($stats.firstUsed)}</p>
    </div>
  </div>

  <!-- Library Stats -->
  <div class="bg-white/5 rounded-xl p-4 mb-6">
    <h3 class="text-lg font-semibold mb-3">Library</h3>
    <div class="grid grid-cols-3 gap-4 text-center">
      <div>
        <p class="text-2xl font-bold text-[#1db954]">{librarySize}</p>
        <p class="text-xs text-white/50">Songs</p>
      </div>
      <div>
        <p class="text-2xl font-bold text-pink-400">{favoritesSize}</p>
        <p class="text-xs text-white/50">Favorites</p>
      </div>
      <div>
        <p class="text-2xl font-bold text-blue-400">{playlistsSize}</p>
        <p class="text-xs text-white/50">Playlists</p>
      </div>
    </div>
  </div>

  <!-- Most Played -->
  {#if topSongs.length > 0}
    <div class="bg-white/5 rounded-xl p-4">
      <h3 class="text-lg font-semibold mb-3">Most Played</h3>
      <div class="space-y-2">
        {#each topSongs as [song, count], i}
          <div class="flex items-center gap-3">
            <span class="w-5 text-center text-white/40 text-sm">{i + 1}</span>
            <div class="flex-1 truncate text-sm">{song}</div>
            <span class="text-xs text-white/50">{count}x</span>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="bg-white/5 rounded-xl p-4 text-center text-white/40">
      <Icon icon="mdi:chart-line" class="w-8 h-8 mx-auto mb-2 opacity-50" />
      <p>No play history yet</p>
      <p class="text-xs">Start playing to see your stats!</p>
    </div>
  {/if}
</div>

