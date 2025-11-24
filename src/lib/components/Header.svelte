<script>
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    isMinimized,
    isDraggable,
    smartPause,
    toggleDraggable,
    loadMidiFiles
  } from '../stores/player.js';

  const appWindow = getCurrentWindow();

  async function minimize() {
    isMinimized.update(v => !v);
  }

  async function close() {
    await appWindow.close();
  }

  function toggleSmartPause() {
    smartPause.update(v => !v);
  }
</script>

<div class="flex items-center justify-between px-6 py-4 border-b border-white/10 {$isDraggable ? 'drag-handle' : ''}">
  <div class="flex items-center gap-3 no-drag">
    <h1 class="text-sm font-semibold tracking-[0.3em] text-white uppercase">WWM MIDI PLAYER</h1>

    <button
      class="px-3 py-1 rounded-full text-[11px] font-medium border transition-colors {$isDraggable ? 'bg-white text-black border-transparent shadow' : 'border-white/20 text-white/70'}"
      on:click={toggleDraggable}
      title="Toggle interaction mode"
    >
      {$isDraggable ? 'Interactive' : 'Click-through'}
    </button>

    <button
      class="px-3 py-1 rounded-full text-[11px] font-medium border transition-colors {$smartPause ? 'border-white/30 text-white' : 'border-white/10 text-white/60'}"
      on:click={toggleSmartPause}
      title="Auto-pause when game loses focus"
    >
      {$smartPause ? 'Smart Pause' : 'Manual'}
    </button>
  </div>

  <div class="flex items-center gap-2 no-drag">
    <button
      class="p-2 rounded-full text-white/60 hover:text-white hover:bg-white/10 transition-colors"
      on:click={minimize}
      title="{$isMinimized ? 'Expand' : 'Minimize'}"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        {#if $isMinimized}
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8h16M4 16h16" />
        {:else}
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
        {/if}
      </svg>
    </button>

    <button
      class="p-2 rounded-full text-white/60 hover:text-white hover:bg-white/10 transition-colors"
      on:click={loadMidiFiles}
      title="Refresh album"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v6h6M20 20v-6h-6M5 19a9 9 0 0114-7.5M19 5a9 9 0 01-14 7.5" />
      </svg>
    </button>

    <button
      class="p-2 rounded-full text-white/60 hover:text-white hover:bg-white/10 transition-colors"
      on:click={close}
      title="Close"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>
</div>
