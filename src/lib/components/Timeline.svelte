<script>
  export let compact = false;

  import {
    currentPosition,
    totalDuration,
    progress,
    formatTime
  } from '../stores/player.js';
</script>

<div class="mt-2 {compact ? 'py-1' : ''}">
  <div class="flex items-center gap-2">
    <span class="text-[11px] text-white/70 min-w-[40px]">
      {formatTime($currentPosition)}
    </span>

    <div
      class="group flex-1 h-1 bg-white/20 rounded-full relative hover:h-1.5 transition-all"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={$progress}
      aria-label="Playback progress"
    >
      <div
        class="h-full bg-white rounded-full relative group-hover:bg-[#1db954]"
        style="width: {$progress}%"
      >
        <!-- Playhead - only visible on hover -->
        <div
          class="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity shadow-lg"
        ></div>
      </div>
    </div>

    <span class="text-[11px] text-white/70 min-w-[40px] text-right">
      {formatTime($totalDuration)}
    </span>
  </div>
</div>
