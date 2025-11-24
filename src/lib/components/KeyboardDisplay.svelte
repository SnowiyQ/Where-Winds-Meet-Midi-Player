<script>
  import {
    LOW_KEYS,
    MID_KEYS,
    HIGH_KEYS,
    activeKeys
  } from '../stores/keyboard.js';

  const noteNames = ['Do', 'Re', 'Mi', 'Fa', 'So', 'La', 'Ti'];
</script>

<div class="space-y-3 p-4 bg-black/20 rounded-lg no-drag">
  <h3 class="text-xs font-semibold text-gray-400 text-center">Keyboard Visualization</h3>

  <!-- High Octave -->
  <div class="flex items-center space-x-2">
    <span class="text-xs text-purple-400 w-12">High:</span>
    <div class="flex space-x-1 flex-1">
      {#each HIGH_KEYS as key, i}
        <div class="flex-1 flex flex-col items-center">
          <span class="text-[10px] text-gray-500 mb-1">{noteNames[i]}</span>
          <button
            class="key-button key-high {$activeKeys.has(key) ? 'key-active' : ''} w-full"
            disabled
          >
            {key.toUpperCase()}
          </button>
        </div>
      {/each}
    </div>
  </div>

  <!-- Mid Octave -->
  <div class="flex items-center space-x-2">
    <span class="text-xs text-green-400 w-12">Mid:</span>
    <div class="flex space-x-1 flex-1">
      {#each MID_KEYS as key, i}
        <div class="flex-1 flex flex-col items-center">
          <span class="text-[10px] text-gray-500 mb-1">{noteNames[i]}</span>
          <button
            class="key-button key-mid {$activeKeys.has(key) ? 'key-active' : ''} w-full"
            disabled
          >
            {key.toUpperCase()}
          </button>
        </div>
      {/each}
    </div>
  </div>

  <!-- Low Octave -->
  <div class="flex items-center space-x-2">
    <span class="text-xs text-blue-400 w-12">Low:</span>
    <div class="flex space-x-1 flex-1">
      {#each LOW_KEYS as key, i}
        <div class="flex-1 flex flex-col items-center">
          <span class="text-[10px] text-gray-500 mb-1">{noteNames[i]}</span>
          <button
            class="key-button key-low {$activeKeys.has(key) ? 'key-active' : ''} w-full"
            disabled
          >
            {key.toUpperCase()}
          </button>
        </div>
      {/each}
    </div>
  </div>
</div>