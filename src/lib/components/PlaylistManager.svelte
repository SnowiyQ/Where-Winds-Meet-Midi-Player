<script>
  import { playlist, currentFile, currentIndex, playMidi, isPlaying, isPaused } from '../stores/player.js';

  let draggedIndex = null;
  let draggedOver = null;

  function handleDragStart(event, index) {
    draggedIndex = index;
    event.dataTransfer.effectAllowed = 'move';
  }

  function handleDragOver(event, index) {
    event.preventDefault();
    draggedOver = index;
  }

  function handleDragEnd() {
    if (draggedIndex !== null && draggedOver !== null && draggedIndex !== draggedOver) {
      playlist.update(list => {
        const items = [...list];
        const draggedItem = items[draggedIndex];

        // Remove dragged item
        items.splice(draggedIndex, 1);

        // Insert at new position
        items.splice(draggedOver, 0, draggedItem);

        return items;
      });
    }

    draggedIndex = null;
    draggedOver = null;
  }

  function removeFromPlaylist(index) {
    playlist.update(list => {
      const items = [...list];
      items.splice(index, 1);
      return items;
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
      <p class="text-sm text-white/60">{$playlist.length} songs • {$currentIndex + 1} currently playing</p>
    </div>
    {#if $playlist.length > 0}
      <button
        class="spotify-button spotify-button--secondary text-xs"
        on:click={clearPlaylist}
      >
        Clear
      </button>
    {/if}
  </div>

  <!-- Playlist Items -->
  <div class="flex-1 overflow-y-auto space-y-1" role="list" aria-live="polite">
    {#each $playlist as file, index}
      <div
        class="group spotify-list-item flex items-center gap-4 py-2 cursor-move {$currentFile === file.path ? 'bg-white/10' : ''} {draggedOver === index ? 'bg-white/5' : ''}"
        draggable="true"
        role="listitem"
        aria-grabbed={draggedIndex === index}
        on:dragstart={(e) => handleDragStart(e, index)}
        on:dragover={(e) => handleDragOver(e, index)}
        on:dragend={handleDragEnd}
      >
        <!-- Drag Handle -->
        <div class="w-6 flex items-center justify-center text-white/40 flex-shrink-0">
          <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
            <path d="M7 2a2 2 0 00-2 2v12a2 2 0 002 2h6a2 2 0 002-2V4a2 2 0 00-2-2H7zm3 14a1 1 0 100-2 1 1 0 000 2zm0-4a1 1 0 100-2 1 1 0 000 2zm0-4a1 1 0 100-2 1 1 0 000 2z" />
          </svg>
        </div>

        <!-- Number / Play Button / Playing Indicator -->
        <div class="w-8 flex items-center justify-center flex-shrink-0">
          {#if $currentFile === file.path && $isPlaying && !$isPaused}
            <!-- Playing indicator (animated bars) -->
            <div class="flex items-end gap-0.5 h-4">
              <div class="w-0.5 bg-[#1db954]" style="height: 60%; animation: music-bar-1 0.6s ease-in-out infinite;"></div>
              <div class="w-0.5 bg-[#1db954]" style="height: 100%; animation: music-bar-2 0.8s ease-in-out infinite;"></div>
              <div class="w-0.5 bg-[#1db954]" style="height: 80%; animation: music-bar-3 0.7s ease-in-out infinite;"></div>
            </div>
          {:else}
            <span class="text-sm text-white/60 {$currentFile === file.path ? 'text-[#1db954]' : ''} group-hover:hidden">{index + 1}</span>
            <button
              class="hidden group-hover:block"
              on:click={() => playFromPlaylist(index)}
              title="Play"
            >
              <svg class="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 24 24">
                <path d="M8 5v14l11-7z"/>
              </svg>
            </button>
          {/if}
        </div>

        <!-- Song Info -->
        <div
          class="flex-1 min-w-0"
          role="button"
          tabindex="0"
          on:click={() => playFromPlaylist(index)}
          on:keydown={(event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); playFromPlaylist(index); } }}
        >
          <p class="text-sm font-medium text-white truncate {$currentFile === file.path ? 'text-[#1db954]' : ''}">{file.name}</p>
          <p class="text-xs text-white/60">MIDI Track</p>
        </div>

        <!-- Duration -->
        <div class="text-sm text-white/60 flex-shrink-0">
          {file.duration ? `${Math.floor(file.duration / 60)}:${String(Math.floor(file.duration % 60)).padStart(2, '0')}` : '--:--'}
        </div>

        <!-- Remove Button -->
        <button
          class="opacity-0 group-hover:opacity-100 text-white/60 hover:text-white transition-all flex-shrink-0"
          on:click={() => removeFromPlaylist(index)}
          title="Remove"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    {/each}

    {#if $playlist.length === 0}
      <div class="text-center py-16 text-white/50">
        <svg class="w-16 h-16 mx-auto mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
        </svg>
        <p class="text-base font-semibold mb-2">Queue is empty</p>
        <p class="text-sm">Add tracks from your library</p>
      </div>
    {/if}
  </div>

  {#if $playlist.length > 0}
    <div class="pt-4 mt-4 border-t border-white/10">
      <p class="text-xs text-white/50 text-center">
        Drag to reorder your queue
      </p>
    </div>
  {/if}
</div>
