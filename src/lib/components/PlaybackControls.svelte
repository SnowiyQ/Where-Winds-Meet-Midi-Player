<script>
  export let compact = false;
  import Icon from '@iconify/svelte';

  import {
    isPlaying,
    isPaused,
    loopMode,
    pauseResume,
    stopPlayback,
    playNext,
    playPrevious,
    toggleLoop
  } from '../stores/player.js';
</script>

<div class="flex items-center justify-center gap-4 {compact ? 'py-2' : ''}">
  <!-- Previous -->
  <button
    class="text-white/60 hover:text-white transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
    on:click={playPrevious}
    disabled={!$isPlaying}
    title="Previous (Ctrl+P)"
  >
    <Icon icon="mdi:skip-previous" class="w-5 h-5" />
  </button>

  <!-- Play/Pause -->
  <button
    class="w-8 h-8 rounded-full bg-white hover:scale-105 transition-transform flex items-center justify-center"
    on:click={pauseResume}
    title="Play/Pause (ScrollLock)"
  >
    {#if $isPlaying && !$isPaused}
      <Icon icon="mdi:pause" class="w-5 h-5 text-black" />
    {:else}
      <Icon icon="mdi:play" class="w-5 h-5 text-black" />
    {/if}
  </button>

  <!-- Next -->
  <button
    class="text-white/60 hover:text-white transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
    on:click={playNext}
    disabled={!$isPlaying}
    title="Next (Ctrl+N)"
  >
    <Icon icon="mdi:skip-next" class="w-5 h-5" />
  </button>
</div>
