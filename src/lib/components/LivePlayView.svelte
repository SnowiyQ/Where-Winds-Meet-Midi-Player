<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { onMount } from "svelte";
  import { invoke } from "../tauri/core-proxy.js";
  import { t } from "svelte-i18n";
  import {
    midiInputDevices,
    selectedMidiDevice,
    selectedMidiDeviceIndex,
    isLiveModeActive,
    isDevVirtualConnected,
    midiConnectionState,
    lastLiveNote,
    refreshMidiDevices,
    startMidiListening,
    stopMidiListening,
    initializeLiveMidiListeners,
  } from "../stores/player.js";

  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

  function handleScroll(e) {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    showTopMask = scrollTop > 10;
    showBottomMask = scrollTop + clientHeight < scrollHeight - 10;
  }

  let isConnecting = false;
  let showDeviceMenu = false;

  // Visual keyboard state
  let activeKeys = new Set();

  // Key layout for visualization
  const keyLayout = {
    high: ["Q", "W", "E", "R", "T", "Y", "U"],
    mid: ["A", "S", "D", "F", "G", "H", "J"],
    low: ["Z", "X", "C", "V", "B", "N", "M"]
  };

  // Connection state styling (reactive for i18n)
  $: stateConfig = {
    NoDevices: { color: "text-zinc-400", bgColor: "bg-zinc-500/10", icon: "mdi:midi-port", message: $t("livePlay.noDevicesFound") },
    DevicesAvailable: { color: "text-yellow-400", bgColor: "bg-yellow-500/10", icon: "mdi:midi", message: $t("livePlay.selectDevice") },
    Connecting: { color: "text-yellow-400", bgColor: "bg-yellow-500/10", icon: "mdi:loading", message: $t("livePlay.connecting") },
    Connected: { color: "text-[#1db954]", bgColor: "bg-[#1db954]/10", icon: "mdi:check-circle", message: $t("livePlay.connected") },
    Disconnected: { color: "text-red-400", bgColor: "bg-red-500/10", icon: "mdi:close-circle", message: $t("livePlay.disconnected") },
    Error: { color: "text-red-400", bgColor: "bg-red-500/10", icon: "mdi:alert-circle", message: $t("livePlay.connectionError") },
  };

  $: currentState = stateConfig[$midiConnectionState] || stateConfig.NoDevices;
  $: selectedDeviceName = $selectedMidiDeviceIndex !== null && deviceList[$selectedMidiDeviceIndex]
    ? deviceList[$selectedMidiDeviceIndex]
    : null;

  // Update active keys based on last note
  $: if ($lastLiveNote) {
    const key = $lastLiveNote.key.toUpperCase().replace("SHIFT+", "").replace("CTRL+", "");
    activeKeys = new Set([key]);
    setTimeout(() => { activeKeys = new Set(); }, 150);
  }

  onMount(async () => {
    initializeLiveMidiListeners();
    // Only refresh devices if not already connected (preserve state across navigation)
    if (!$isLiveModeActive && !$isDevVirtualConnected) {
      await refreshMidiDevices();
    }
  });

  async function handleRefresh() {
    await refreshMidiDevices();
  }

  function selectDevice(index) {
    selectedMidiDeviceIndex.set(index);
    showDeviceMenu = false;
  }

  // DEV mode detection
  const isDev = import.meta.env.DEV;
  const DEV_VIRTUAL_DEVICE = "🎹 DEV: Virtual MIDI Keyboard";

  // Combined device list with virtual device in dev mode
  $: deviceList = isDev ? [DEV_VIRTUAL_DEVICE, ...$midiInputDevices] : $midiInputDevices;
  $: isVirtualDeviceSelected = isDev && $selectedMidiDeviceIndex === 0;
  $: isListening = $isLiveModeActive || $isDevVirtualConnected;

  // Handle connect for virtual device
  async function handleConnectWithVirtual() {
    if ($selectedMidiDeviceIndex === null) return;

    if (isVirtualDeviceSelected) {
      // Virtual device - just set connected state
      isDevVirtualConnected.set(true);
      midiConnectionState.set('Connected');
    } else {
      // Real device - adjust index for the virtual device offset
      isConnecting = true;
      const realIndex = isDev ? $selectedMidiDeviceIndex - 1 : $selectedMidiDeviceIndex;
      await startMidiListening(realIndex);
      isConnecting = false;
    }
  }

  // Handle disconnect for virtual device
  async function handleDisconnectWithVirtual() {
    if (isVirtualDeviceSelected || $isDevVirtualConnected) {
      isDevVirtualConnected.set(false);
      midiConnectionState.set($midiInputDevices.length > 0 ? 'DevicesAvailable' : 'NoDevices');
      selectedMidiDeviceIndex.set(null);
    } else {
      await stopMidiListening();
      selectedMidiDeviceIndex.set(null);
    }
  }

  // Direct key mapping for virtual piano (bypasses MIDI mapping, like test_all_keys_36)
  // 3 octaves with white keys (natural) and black keys (sharps/flats with modifiers)
  const pianoRows = [
    { // Low octave (Z-M row)
      label: "Low",
      white: [
        { key: "z", note: "C3" },
        { key: "x", note: "D3" },
        { key: "c", note: "E3" },
        { key: "v", note: "F3" },
        { key: "b", note: "G3" },
        { key: "n", note: "A3" },
        { key: "m", note: "B3" },
      ],
      black: [
        { key: "shift+z", note: "C#3", pos: 0 },
        { key: "ctrl+c", note: "Eb3", pos: 1 },
        { key: "shift+v", note: "F#3", pos: 3 },
        { key: "shift+b", note: "G#3", pos: 4 },
        { key: "ctrl+m", note: "Bb3", pos: 5 },
      ],
    },
    { // Mid octave (A-J row)
      label: "Mid",
      white: [
        { key: "a", note: "C4" },
        { key: "s", note: "D4" },
        { key: "d", note: "E4" },
        { key: "f", note: "F4" },
        { key: "g", note: "G4" },
        { key: "h", note: "A4" },
        { key: "j", note: "B4" },
      ],
      black: [
        { key: "shift+a", note: "C#4", pos: 0 },
        { key: "ctrl+d", note: "Eb4", pos: 1 },
        { key: "shift+f", note: "F#4", pos: 3 },
        { key: "shift+g", note: "G#4", pos: 4 },
        { key: "ctrl+j", note: "Bb4", pos: 5 },
      ],
    },
    { // High octave (Q-U row)
      label: "High",
      white: [
        { key: "q", note: "C5" },
        { key: "w", note: "D5" },
        { key: "e", note: "E5" },
        { key: "r", note: "F5" },
        { key: "t", note: "G5" },
        { key: "y", note: "A5" },
        { key: "u", note: "B5" },
      ],
      black: [
        { key: "shift+q", note: "C#5", pos: 0 },
        { key: "ctrl+e", note: "Eb5", pos: 1 },
        { key: "shift+r", note: "F#5", pos: 3 },
        { key: "shift+t", note: "G#5", pos: 4 },
        { key: "ctrl+u", note: "Bb5", pos: 5 },
      ],
    },
  ];

  // Direct tap key (bypasses MIDI mapping)
  async function tapKey(key) {
    try {
      await invoke('tap_key', { key });
      // Update visual feedback
      const displayKey = key.toUpperCase().replace("SHIFT+", "").replace("CTRL+", "");
      activeKeys = new Set([displayKey]);
      setTimeout(() => { activeKeys = new Set(); }, 150);
    } catch (e) {
      console.error('Failed to tap key:', e);
    }
  }

  // Pick a random key from the piano and tap it
  function simulateRandomNote() {
    const allKeys = pianoRows.flatMap(row => [
      ...row.white.map(k => k.key),
      ...row.black.map(k => k.key)
    ]);
    const randomKey = allKeys[Math.floor(Math.random() * allKeys.length)];
    tapKey(randomKey);
  }
</script>

<div
  bind:this={scrollContainer}
  onscroll={handleScroll}
  class="h-full overflow-y-auto space-y-4 {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
>
  <!-- Header -->
  <div class="mb-2">
    <h2 class="text-2xl font-bold">{$t("livePlay.title")}</h2>
    <p class="text-sm text-white/60">{$t("livePlay.subtitle")}</p>
  </div>

  <!-- Device Selection -->
  <div class="bg-white/5 rounded-lg p-4 space-y-3">
    <div class="flex items-center gap-2 text-white/70 text-sm font-medium">
      <Icon icon="mdi:usb" class="w-4 h-4" />
      <span>{$t("livePlay.midiDevice")}</span>
    </div>

    <!-- Device Dropdown -->
    <div class="flex gap-2">
      <div class="relative flex-1">
        <button
          class="w-full flex items-center justify-between gap-2 px-3 py-2.5 bg-white/5 hover:bg-white/10 rounded-lg text-sm transition-colors {isListening ? 'opacity-50 cursor-not-allowed' : ''}"
          onclick={() => !isListening && (showDeviceMenu = !showDeviceMenu)}
          disabled={isListening}
        >
          <div class="flex items-center gap-2 min-w-0">
            <Icon icon={isVirtualDeviceSelected ? 'mdi:piano' : 'mdi:midi'} class="w-4 h-4 {isVirtualDeviceSelected ? 'text-orange-400' : 'text-white/50'} flex-shrink-0" />
            <span class="truncate {selectedDeviceName ? (isVirtualDeviceSelected ? 'text-orange-400' : 'text-white') : 'text-white/50'}">
              {selectedDeviceName || $t("livePlay.selectDevice") + '...'}
            </span>
          </div>
          <Icon icon="mdi:chevron-down" class="w-4 h-4 text-white/50 flex-shrink-0" />
        </button>

        {#if showDeviceMenu}
          <button class="fixed inset-0 z-40" onclick={() => showDeviceMenu = false}></button>
          <div
            class="absolute top-full left-0 right-0 mt-1 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 max-h-60 overflow-y-auto"
            in:fly={{ y: -10, duration: 150 }}
            out:fade={{ duration: 100 }}
          >
            {#if deviceList.length === 0}
              <div class="px-3 py-3 text-sm text-white/40 text-center">
                {$t("livePlay.noDevicesFound")}
              </div>
            {:else}
              {#each deviceList as device, index}
                <button
                  class="w-full flex items-center gap-2 px-3 py-2.5 text-left text-sm transition-colors {$selectedMidiDeviceIndex === index ? 'bg-[#1db954]/20' : 'hover:bg-white/5'}"
                  onclick={() => selectDevice(index)}
                >
                  <Icon icon={isDev && index === 0 ? 'mdi:piano' : 'mdi:midi'} class="w-4 h-4 {$selectedMidiDeviceIndex === index ? 'text-[#1db954]' : isDev && index === 0 ? 'text-orange-400' : 'text-white/50'}" />
                  <span class="flex-1 truncate {$selectedMidiDeviceIndex === index ? 'text-[#1db954]' : isDev && index === 0 ? 'text-orange-400' : 'text-white/90'}">{device}</span>
                  {#if $selectedMidiDeviceIndex === index}
                    <Icon icon="mdi:check" class="w-4 h-4 text-[#1db954]" />
                  {/if}
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>

      <button
        class="p-2.5 bg-white/5 hover:bg-white/10 rounded-lg transition-colors {isListening ? 'opacity-50 cursor-not-allowed' : ''}"
        onclick={handleRefresh}
        disabled={isListening}
        title={$t("livePlay.refreshDevices")}
      >
        <Icon icon="mdi:refresh" class="w-5 h-5" />
      </button>
    </div>

    <!-- Status -->
    <div class="flex items-center gap-2 px-3 py-2 rounded-lg {currentState.bgColor}">
      <Icon icon={currentState.icon} class="w-4 h-4 {currentState.color} {$midiConnectionState === 'Connecting' ? 'animate-spin' : ''}" />
      <span class="text-sm {currentState.color}">{currentState.message}</span>
      {#if $selectedMidiDevice}
        <span class="text-sm text-white font-medium">: {$selectedMidiDevice.name}</span>
      {/if}
    </div>

    <!-- Connect Button -->
    {#if isListening}
      <button
        class="w-full flex items-center justify-center gap-2 px-4 py-3 bg-red-500 hover:bg-red-600 rounded-lg font-medium transition-colors"
        onclick={handleDisconnectWithVirtual}
      >
        <Icon icon="mdi:stop" class="w-5 h-5" />
        <span>{$t("livePlay.stopListening")}</span>
      </button>
    {:else}
      <button
        class="w-full flex items-center justify-center gap-2 px-4 py-3 bg-[#1db954] hover:bg-[#1ed760] disabled:bg-white/10 disabled:text-white/30 rounded-lg font-medium transition-colors"
        onclick={handleConnectWithVirtual}
        disabled={$selectedMidiDeviceIndex === null || isConnecting}
      >
        <Icon icon="mdi:play" class="w-5 h-5" />
        <span>{isConnecting ? $t("livePlay.connecting") : $t("livePlay.startListening")}</span>
      </button>
    {/if}
  </div>

  <!-- Visual Feedback (only for real MIDI devices, not DEV virtual) -->
  {#if $isLiveModeActive && !$isDevVirtualConnected}
    <div class="bg-white/5 rounded-lg p-4 space-y-3" in:fly={{ y: 10, duration: 200 }}>
      <div class="flex items-center gap-2 text-white/70 text-sm font-medium">
        <Icon icon="mdi:music-note" class="w-4 h-4" />
        <span>{$t("livePlay.liveInput")}</span>
      </div>

      <!-- Last Note Display -->
      <div class="bg-black/30 rounded-lg p-4 text-center min-h-[60px] flex items-center justify-center">
        {#if $lastLiveNote}
          <div class="flex items-center gap-3 text-lg" in:fade={{ duration: 100 }}>
            <span class="text-[#1db954] font-bold text-2xl">{$lastLiveNote.noteName}</span>
            <Icon icon="mdi:arrow-right" class="w-5 h-5 text-white/50" />
            <span class="text-white font-medium">{$t("livePlay.key")} {$lastLiveNote.key.toUpperCase()}</span>
          </div>
        {:else}
          <span class="text-white/40 italic">{$isDevVirtualConnected ? $t("livePlay.waitingForInput") : $t("livePlay.playNoteOnDevice")}</span>
        {/if}
      </div>

      <!-- Visual Keyboard -->
      <div class="space-y-1">
        {#each ["high", "mid", "low"] as row}
          <div class="flex items-center gap-1">
            <span class="w-10 text-xs text-white/40 text-right pr-2">{row.charAt(0).toUpperCase() + row.slice(1)}</span>
            {#each keyLayout[row] as key}
              <div
                class="flex-1 aspect-square max-w-[36px] flex items-center justify-center rounded text-sm font-medium transition-all duration-100
                  {activeKeys.has(key) ? 'bg-[#1db954] text-white scale-110 shadow-lg shadow-[#1db954]/30' : 'bg-white/10 text-white/50 border border-white/10'}"
              >
                {key}
              </div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Info -->
  <div class="flex gap-3 p-3 bg-blue-500/10 border border-blue-500/20 rounded-lg text-blue-400 text-xs">
    <Icon icon="mdi:information-outline" class="w-5 h-5 flex-shrink-0" />
    <div class="space-y-1">
      <p>{$t("livePlay.info1")}</p>
      <p>{$t("livePlay.info2")}</p>
    </div>
  </div>

  <!-- DEV: Virtual Piano Keyboard (only when virtual device connected) -->
  {#if $isDevVirtualConnected}
    <div class="bg-orange-500/10 border border-orange-500/20 rounded-xl p-4 space-y-4" in:fly={{ y: 10, duration: 200 }}>
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2 text-orange-400 text-sm font-medium">
          <Icon icon="mdi:piano" class="w-5 h-5" />
          <span>{$t("livePlay.virtualKeyboard")}</span>
        </div>
        <span class="text-[10px] text-orange-400/50 bg-orange-500/20 px-2 py-0.5 rounded">{$t("livePlay.devOnly")}</span>
      </div>

      <!-- Piano Keyboard (direct key mapping like test_all_keys_36) -->
      <div class="flex justify-center">
        <div class="flex gap-0.5">
          {#each pianoRows as row, rowIdx}
            <div class="relative">
              <!-- White keys -->
              <div class="flex">
                {#each row.white as wkey, keyIdx}
                  <button
                    class="w-8 h-24 bg-white hover:bg-gray-100 active:bg-gray-200 border border-gray-300 rounded-b-md transition-colors relative z-0"
                    onclick={() => tapKey(wkey.key)}
                    title="{wkey.note} → {wkey.key.toUpperCase()}"
                  >
                    <span class="absolute bottom-1 left-1/2 -translate-x-1/2 text-[9px] text-gray-400 font-mono">
                      {wkey.key.toUpperCase()}
                    </span>
                  </button>
                {/each}
              </div>
              <!-- Black keys (sharps/flats with Shift/Ctrl) -->
              <div class="absolute top-0 left-0 pointer-events-none">
                {#each row.black as bkey}
                  <button
                    class="absolute w-5 h-14 bg-gray-900 hover:bg-gray-700 active:bg-gray-600 rounded-b-md transition-colors z-10 pointer-events-auto border border-gray-800 flex items-end justify-center pb-1"
                    style="left: {(bkey.pos + 0.7) * 32}px"
                    onclick={() => tapKey(bkey.key)}
                    title="{bkey.note} → {bkey.key.toUpperCase()}"
                  >
                    <span class="text-[7px] text-gray-500 font-mono leading-none">
                      {bkey.key.includes('shift') ? 'S' : 'C'}
                    </span>
                  </button>
                {/each}
              </div>
              <!-- Row label -->
              <div class="text-center mt-1">
                <span class="text-[9px] text-orange-400/50">{row.label}</span>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Quick buttons -->
      <div class="flex items-center justify-center gap-2 pt-2 border-t border-orange-500/20">
        <button
          class="px-3 py-1.5 bg-orange-500/20 hover:bg-orange-500/30 text-orange-300 rounded-lg text-xs transition-colors"
          onclick={simulateRandomNote}
        >
          <Icon icon="mdi:dice-5" class="w-4 h-4 inline mr-1" />
          {$t("livePlay.randomNote")}
        </button>
        <span class="text-[10px] text-orange-400/40">{$t("livePlay.clickKeysToSimulate")}</span>
      </div>
    </div>
  {/if}
</div>
