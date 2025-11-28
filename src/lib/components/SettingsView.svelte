<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import {
    noteMode,
    setNoteMode,
    keyMode,
    setKeyMode,
    modifierDelay,
    setModifierDelay,
    testAllKeys,
    testAllKeys36,
    smartPause,
    loadMidiFiles,
  } from "../stores/player.js";

  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

  function handleScroll(e) {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    showTopMask = scrollTop > 10;
    showBottomMask = scrollTop + clientHeight < scrollHeight - 10;
  }

  let isTesting = false;
  let isTesting36 = false;
  let isSpamming = false;
  let isSpammingMulti = false;
  let isSpammingChord = false;
  let spamKey = "a";
  let spamCount = 50;
  let spamDelay = 20;
  let chordSize = 3;
  let cloudMode = false;
  let albumPath = "";
  let isChangingPath = false;

  const APP_VERSION = "1.0.6";
  let updateAvailable = null;

  const isDev = import.meta.env.DEV;

  onMount(async () => {
    // Check initial scroll state
    setTimeout(() => {
      if (scrollContainer) {
        const { scrollHeight, clientHeight } = scrollContainer;
        showBottomMask = scrollHeight > clientHeight;
      }
    }, 100);

    // Load cloud mode
    try {
      cloudMode = await invoke('get_cloud_mode');
    } catch (e) {
      console.error("Failed to get cloud mode:", e);
    }

    // Load album path
    try {
      albumPath = await invoke('get_album_path');
    } catch (e) {
      console.error("Failed to get album path:", e);
    }

    // Check for updates
    checkForUpdates();
  });

  async function checkForUpdates() {
    try {
      const response = await fetch('https://api.github.com/repos/SnowiyQ/Where-Winds-Meet-Midi-Player/releases/latest');
      if (!response.ok) return;
      const data = await response.json();
      const latestVersion = data.tag_name?.replace(/^v/, '') || '';
      if (latestVersion && compareVersions(latestVersion, APP_VERSION) > 0) {
        updateAvailable = {
          version: latestVersion,
          url: data.html_url
        };
      }
    } catch (e) {
      console.log('Update check failed:', e);
    }
  }

  function compareVersions(a, b) {
    const partsA = a.split('.').map(Number);
    const partsB = b.split('.').map(Number);
    for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
      const numA = partsA[i] || 0;
      const numB = partsB[i] || 0;
      if (numA > numB) return 1;
      if (numA < numB) return -1;
    }
    return 0;
  }

  async function toggleCloudMode() {
    cloudMode = !cloudMode;
    try {
      await invoke('set_cloud_mode', { enabled: cloudMode });
    } catch (e) {
      console.error("Failed to set cloud mode:", e);
      cloudMode = !cloudMode; // revert on error
    }
  }

  async function changeAlbumPath() {
    if (isChangingPath) return;
    isChangingPath = true;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Album Folder",
      });
      if (selected) {
        await invoke('set_album_path', { path: selected });
        albumPath = selected;
        await loadMidiFiles(); // Reload files from new location
      }
    } catch (e) {
      console.error("Failed to change album path:", e);
    } finally {
      isChangingPath = false;
    }
  }

  async function resetAlbumPath() {
    try {
      albumPath = await invoke('reset_album_path');
      await loadMidiFiles(); // Reload files from default location
    } catch (e) {
      console.error("Failed to reset album path:", e);
    }
  }

  async function handleSpamTest() {
    if (isSpamming) return;
    isSpamming = true;
    try {
      await invoke('spam_test', {
        key: spamKey,
        count: parseInt(spamCount),
        delayMs: parseInt(spamDelay)
      });
    } catch (error) {
      console.error("Spam test failed:", error);
    } finally {
      isSpamming = false;
    }
  }

  async function handleSpamTestMulti() {
    if (isSpammingMulti) return;
    isSpammingMulti = true;
    try {
      await invoke('spam_test_multi', {
        count: parseInt(spamCount),
        delayMs: parseInt(spamDelay)
      });
    } catch (error) {
      console.error("Multi spam test failed:", error);
    } finally {
      isSpammingMulti = false;
    }
  }

  async function handleSpamTestChord() {
    if (isSpammingChord) return;
    isSpammingChord = true;
    try {
      await invoke('spam_test_chord', {
        chordSize: parseInt(chordSize),
        count: parseInt(spamCount),
        delayMs: parseInt(spamDelay)
      });
    } catch (error) {
      console.error("Chord test failed:", error);
    } finally {
      isSpammingChord = false;
    }
  }

  // Note calculation mode options
  const noteModes = [
    {
      id: "Python",
      name: "YueLyn (Recommended)",
      description: "YueLyn most fav play mode",
    },
    {
      id: "Closest",
      name: "Closest",
      description: "Find closest available note (original, best for most songs)",
    },
    {
      id: "Quantize",
      name: "Quantize",
      description: "Snap to exact scale notes only",
    },
    {
      id: "TransposeOnly",
      name: "Transpose Only",
      description: "Direct mapping with octave shifting",
    },
    {
      id: "Pentatonic",
      name: "Pentatonic",
      description: "Map to 5-note pentatonic scale (do-re-mi-so-la)",
    },
    {
      id: "Chromatic",
      name: "Chromatic",
      description: "Detailed 12-semitone to 7-key mapping",
    },
    {
      id: "Raw",
      name: "Raw",
      description: "Direct 1:1 mapping, no auto-transpose (MIDI note % 21)",
    },
  ];

  async function handleModeChange(mode) {
    await setNoteMode(mode);
  }

  function toggleSmartPause() {
    smartPause.update((v) => !v);
  }

  async function handleTestKeys() {
    if (isTesting) return;
    isTesting = true;
    try {
      await testAllKeys();
    } catch (error) {
      console.error("Failed to test keys:", error);
    } finally {
      isTesting = false;
    }
  }

  async function handleTestKeys36() {
    if (isTesting36) return;
    isTesting36 = true;
    try {
      await testAllKeys36();
    } catch (error) {
      console.error("Failed to test 36 keys:", error);
    } finally {
      isTesting36 = false;
    }
  }
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="mb-6">
    <div class="flex items-center gap-4">
      <div>
        <h2 class="text-2xl font-bold">Settings</h2>
        <p class="text-sm text-white/60">Configure your playback preferences</p>
      </div>
    </div>
  </div>

  <!-- Settings Sections -->
  <div
    bind:this={scrollContainer}
    onscroll={handleScroll}
    class="flex-1 overflow-y-auto space-y-6 pr-2 {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
  >
    <!-- Note Mode Section -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 50 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:music-note" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Note Calculation Mode</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        Choose how MIDI notes are mapped to game keys
      </p>

      <div class="space-y-3">
        {#each noteModes as mode}
          <button
            class="w-full p-4 rounded-lg border-2 transition-all duration-200 text-left {$noteMode ===
            mode.id
              ? 'border-[#1db954] bg-[#1db954]/10'
              : 'border-white/10 bg-white/5 hover:border-white/20'}"
            onclick={() => handleModeChange(mode.id)}
          >
            <div class="flex items-center justify-between mb-2">
              <span class="font-semibold text-white">{mode.name}</span>
              {#if $noteMode === mode.id}
                <Icon icon="mdi:check-circle" class="w-5 h-5 text-[#1db954]" />
              {:else}
                <div
                  class="w-5 h-5 rounded-full border-2 border-white/30"
                ></div>
              {/if}
            </div>
            <p class="text-sm text-white/60">{mode.description}</p>
          </button>
        {/each}
      </div>

    </div>

    <!-- Key Mode Section (Play Style) -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 75 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:piano" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Play Style (Key Mode)</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        Choose between 21-key (natural notes) or 36-key (with sharps/flats)
      </p>

      <div class="flex gap-3">
        <button
          class="flex-1 p-4 rounded-lg border-2 transition-all duration-200 {$keyMode === 'Keys21'
            ? 'border-[#1db954] bg-[#1db954]/10'
            : 'border-white/10 bg-white/5 hover:border-white/20'}"
          onclick={() => setKeyMode('Keys21')}
        >
          <div class="text-center">
            <span class="text-2xl font-bold">21</span>
            <p class="text-xs text-white/60 mt-1">Natural notes</p>
          </div>
        </button>
        <button
          class="flex-1 p-4 rounded-lg border-2 transition-all duration-200 {$keyMode === 'Keys36'
            ? 'border-[#1db954] bg-[#1db954]/10'
            : 'border-white/10 bg-white/5 hover:border-white/20'}"
          onclick={() => setKeyMode('Keys36')}
        >
          <div class="text-center">
            <span class="text-2xl font-bold">36</span>
            <p class="text-xs text-white/60 mt-1">With sharps/flats</p>
          </div>
        </button>
      </div>

      <!-- Test Buttons -->
      <div class="mt-4 pt-4 border-t border-white/10 flex gap-3">
        <button
          class="flex-1 py-3 px-4 rounded-lg bg-white/10 hover:bg-white/15 transition-colors flex items-center justify-center gap-2 {isTesting
            ? 'opacity-50 cursor-not-allowed'
            : ''}"
          onclick={handleTestKeys}
          disabled={isTesting}
        >
          <Icon
            icon={isTesting ? "mdi:loading" : "mdi:piano"}
            class="w-5 h-5 {isTesting ? 'animate-spin' : ''}"
          />
          <span class="font-medium text-sm">{isTesting ? "Testing..." : "Test 21"}</span>
        </button>
        <button
          class="flex-1 py-3 px-4 rounded-lg bg-white/10 hover:bg-white/15 transition-colors flex items-center justify-center gap-2 {isTesting36
            ? 'opacity-50 cursor-not-allowed'
            : ''}"
          onclick={handleTestKeys36}
          disabled={isTesting36}
        >
          <Icon
            icon={isTesting36 ? "mdi:loading" : "mdi:piano"}
            class="w-5 h-5 {isTesting36 ? 'animate-spin' : ''}"
          />
          <span class="font-medium text-sm">{isTesting36 ? "Testing..." : "Test 36"}</span>
        </button>
      </div>

      <!-- Modifier Delay (for 36-key mode) -->
      <div class="mt-4 pt-4 border-t border-white/10">
        <div class="flex items-center justify-between mb-2">
          <div>
            <p class="font-medium text-white text-sm">Modifier Delay</p>
            <p class="text-xs text-white/60">Timing for Shift/Ctrl keys (sharps/flats in 36-key mode)</p>
          </div>
          <span class="text-sm font-mono text-[#1db954]">{$modifierDelay}ms</span>
        </div>
        <input
          type="range"
          min="0"
          max="20"
          step="1"
          value={$modifierDelay}
          oninput={(e) => setModifierDelay(parseInt(e.target.value))}
          class="w-full h-2 bg-white/10 rounded-lg appearance-none cursor-pointer accent-[#1db954]"
        />
        <div class="flex justify-between text-xs text-white/40 mt-1">
          <span>0ms (fast)</span>
          <span>20ms (slow)</span>
        </div>
      </div>

      <!-- Spam Test (Dev Only) -->
      {#if isDev}
        <div class="mt-4 pt-4 border-t border-white/10">
          <div class="flex items-center gap-2 mb-3">
            <Icon icon="mdi:bug" class="w-4 h-4 text-orange-400" />
            <p class="font-medium text-orange-400 text-sm">Spam Test (Dev)</p>
          </div>
          <div class="grid grid-cols-3 gap-2 mb-3">
            <div>
              <label class="text-xs text-white/60">Key</label>
              <select
                bind:value={spamKey}
                class="w-full mt-1 px-2 py-1 bg-white/10 rounded text-sm"
              >
                <option value="z">Z</option>
                <option value="a">A</option>
                <option value="q">Q</option>
                <option value="c">C</option>
              </select>
            </div>
            <div>
              <label class="text-xs text-white/60">Count</label>
              <input
                type="number"
                bind:value={spamCount}
                min="1"
                max="200"
                class="w-full mt-1 px-2 py-1 bg-white/10 rounded text-sm"
              />
            </div>
            <div>
              <label class="text-xs text-white/60">Delay (ms)</label>
              <input
                type="number"
                bind:value={spamDelay}
                min="0"
                max="200"
                class="w-full mt-1 px-2 py-1 bg-white/10 rounded text-sm"
              />
            </div>
          </div>
          <div class="grid grid-cols-4 gap-2 mb-2">
            <div>
              <label class="text-xs text-white/60">Chord</label>
              <input
                type="number"
                bind:value={chordSize}
                min="2"
                max="21"
                class="w-full mt-1 px-2 py-1 bg-white/10 rounded text-sm"
              />
            </div>
          </div>
          <div class="flex gap-2">
            <button
              class="flex-1 py-2 px-3 rounded-lg bg-orange-500/20 hover:bg-orange-500/30 border border-orange-500/50 transition-colors flex items-center justify-center gap-1 {isSpamming ? 'opacity-50 cursor-not-allowed' : ''}"
              onclick={handleSpamTest}
              disabled={isSpamming}
            >
              <Icon icon={isSpamming ? "mdi:loading" : "mdi:flash"} class="w-4 h-4 {isSpamming ? 'animate-spin' : ''}" />
              <span class="font-medium text-xs">{isSpamming ? "..." : "1Key"}</span>
            </button>
            <button
              class="flex-1 py-2 px-3 rounded-lg bg-purple-500/20 hover:bg-purple-500/30 border border-purple-500/50 transition-colors flex items-center justify-center gap-1 {isSpammingMulti ? 'opacity-50 cursor-not-allowed' : ''}"
              onclick={handleSpamTestMulti}
              disabled={isSpammingMulti}
            >
              <Icon icon={isSpammingMulti ? "mdi:loading" : "mdi:flash-circle"} class="w-4 h-4 {isSpammingMulti ? 'animate-spin' : ''}" />
              <span class="font-medium text-xs">{isSpammingMulti ? "..." : "21Key"}</span>
            </button>
            <button
              class="flex-1 py-2 px-3 rounded-lg bg-green-500/20 hover:bg-green-500/30 border border-green-500/50 transition-colors flex items-center justify-center gap-1 {isSpammingChord ? 'opacity-50 cursor-not-allowed' : ''}"
              onclick={handleSpamTestChord}
              disabled={isSpammingChord}
            >
              <Icon icon={isSpammingChord ? "mdi:loading" : "mdi:music"} class="w-4 h-4 {isSpammingChord ? 'animate-spin' : ''}" />
              <span class="font-medium text-xs">{isSpammingChord ? "..." : "Chord"}</span>
            </button>
          </div>
        </div>
      {/if}
    </div>

    <!-- Keyboard Layout Info -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 100 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:keyboard" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Keyboard Layout</h3>
      </div>

      <div class="space-y-3 text-sm">
        <!-- 21-Key Layout -->
        <div class="bg-white/5 rounded-lg p-3">
          <p class="font-semibold text-white mb-2">21 Keys (Natural Notes)</p>
          <div class="grid grid-cols-3 gap-2 text-xs">
            <div>
              <span class="text-white/40">High:</span>
              <span class="font-mono">Q W E R T Y U</span>
            </div>
            <div>
              <span class="text-white/40">Mid:</span>
              <span class="font-mono">A S D F G H J</span>
            </div>
            <div>
              <span class="text-white/40">Low:</span>
              <span class="font-mono">Z X C V B N M</span>
            </div>
          </div>
        </div>

        <!-- 36-Key Layout -->
        <div class="bg-white/5 rounded-lg p-3">
          <p class="font-semibold text-white mb-2">+15 Keys (Sharps/Flats)</p>
          <div class="space-y-2 text-xs">
            <div>
              <span class="text-orange-400">Shift+</span>
              <span class="text-white/60">C# F# G#:</span>
              <span class="font-mono text-white/80">Shift+Q/R/T, A/F/G, Z/V/B</span>
            </div>
            <div>
              <span class="text-blue-400">Ctrl+</span>
              <span class="text-white/60">Eb Bb:</span>
              <span class="font-mono text-white/80">Ctrl+E/U, D/J, C/M</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Playback Settings Section -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 150 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:play-circle-outline" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Playback Settings</h3>
      </div>

      <!-- Smart Pause Toggle -->
      <div class="flex items-center justify-between py-3">
        <div>
          <p class="font-medium text-white">Smart Pause</p>
          <p class="text-sm text-white/60">Auto-pause when game loses focus</p>
        </div>
        <button
          class="relative w-12 h-6 rounded-full transition-colors duration-200 {$smartPause
            ? 'bg-[#1db954]'
            : 'bg-white/20'}"
          onclick={toggleSmartPause}
        >
          <div
            class="absolute top-1 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {$smartPause
              ? 'translate-x-7'
              : 'translate-x-1'}"
          ></div>
        </button>
      </div>

      <!-- Cloud Gaming Mode Toggle -->
      <div class="flex items-center justify-between py-3 border-t border-white/10">
        <div>
          <p class="font-medium text-white">Cloud Gaming Mode</p>
          <p class="text-sm text-white/60">For GeForce Now, Xbox Cloud, etc.</p>
          {#if cloudMode}
            <div class="text-xs text-orange-400 mt-1 space-y-0.5">
              <p>⚠️ Uses SendInput (global keyboard simulation)</p>
              <p>⚠️ Background play without focus is NOT possible</p>
              <p>⚠️ Don't type while playing - keys will interfere!</p>
            </div>
          {/if}
        </div>
        <button
          class="relative w-12 h-6 rounded-full transition-colors duration-200 {cloudMode
            ? 'bg-orange-500'
            : 'bg-white/20'}"
          onclick={toggleCloudMode}
        >
          <div
            class="absolute top-1 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {cloudMode
              ? 'translate-x-7'
              : 'translate-x-1'}"
          ></div>
        </button>
      </div>
    </div>

    <!-- Album Location Section -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 175 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:folder-music" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Album Location</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        Choose where to load MIDI files from
      </p>

      <!-- Current Path Display -->
      <div class="bg-white/5 rounded-lg p-3 mb-4">
        <p class="text-xs text-white/40 mb-1">Current folder:</p>
        <p class="text-sm text-white font-mono truncate" title={albumPath}>
          {albumPath || "Loading..."}
        </p>
      </div>

      <!-- Action Buttons -->
      <div class="flex gap-3">
        <button
          class="flex-1 py-3 px-4 rounded-lg bg-white/10 hover:bg-white/15 transition-colors flex items-center justify-center gap-2 {isChangingPath ? 'opacity-50 cursor-not-allowed' : ''}"
          onclick={changeAlbumPath}
          disabled={isChangingPath}
        >
          <Icon
            icon={isChangingPath ? "mdi:loading" : "mdi:folder-open"}
            class="w-5 h-5 {isChangingPath ? 'animate-spin' : ''}"
          />
          <span class="font-medium text-sm">{isChangingPath ? "Selecting..." : "Browse"}</span>
        </button>
        <button
          class="py-3 px-4 rounded-lg bg-white/10 hover:bg-white/15 transition-colors flex items-center justify-center gap-2"
          onclick={resetAlbumPath}
          title="Reset to default (./album)"
        >
          <Icon icon="mdi:restore" class="w-5 h-5" />
          <span class="font-medium text-sm">Reset</span>
        </button>
      </div>
    </div>

    <!-- About Section -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 200 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:information-outline" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">About</h3>
      </div>

      <div class="text-sm text-white/60 space-y-3">
        <p>Midi Player for Where Winds Meet</p>
        <div class="flex items-center gap-2">
          <span class="text-xs text-white/40">By YueLyn</span>
          <span class="text-white/20">•</span>
          <span class="text-xs text-white/40">v{APP_VERSION}</span>
        </div>

        {#if updateAvailable}
          <button
            class="w-full flex items-center gap-3 p-3 rounded-lg bg-[#1db954]/10 hover:bg-[#1db954]/20 transition-colors"
            onclick={() => invoke('open_url', { url: updateAvailable.url })}
          >
            <Icon icon="mdi:download-circle" class="w-6 h-6 text-[#1db954]" />
            <div class="text-left">
              <p class="text-sm font-medium text-[#1db954]">Update Available</p>
              <p class="text-xs text-white/50">v{updateAvailable.version} - Click to download</p>
            </div>
          </button>
        {:else}
          <div class="flex items-center gap-2 text-xs text-white/40">
            <Icon icon="mdi:check-circle" class="w-4 h-4 text-[#1db954]" />
            <span>You're on the latest version</span>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>
