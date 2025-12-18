<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { invoke } from "../tauri/core-proxy.js";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, onDestroy } from "svelte";
  import { t } from "svelte-i18n";
  import {
    noteMode,
    setNoteMode,
    keyMode,
    setKeyMode,
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

  // Note key bindings (customizable keyboard layout)
  let noteKeys = {
    low: ["z", "x", "c", "v", "b", "n", "m"],
    mid: ["a", "s", "d", "f", "g", "h", "j"],
    high: ["q", "w", "e", "r", "t", "y", "u"]
  };
  let recordingNoteKey = null; // { octave: "low"|"mid"|"high", index: 0-6 }

  // Keyboard layout presets
  const KEY_PRESETS = {
    qwerty: {
      name: "QWERTY",
      desc: "US/International layout",
      low: ["z", "x", "c", "v", "b", "n", "m"],
      mid: ["a", "s", "d", "f", "g", "h", "j"],
      high: ["q", "w", "e", "r", "t", "y", "u"]
    },
    qwertz: {
      name: "QWERTZ",
      desc: "German/Austrian layout",
      low: ["y", "x", "c", "v", "b", "n", "m"],
      mid: ["a", "s", "d", "f", "g", "h", "j"],
      high: ["q", "w", "e", "r", "t", "z", "u"]
    },
    azerty: {
      name: "AZERTY",
      desc: "French layout",
      low: ["w", "x", "c", "v", "b", "n", ","],
      mid: ["q", "s", "d", "f", "g", "h", "j"],
      high: ["a", "z", "e", "r", "t", "y", "u"]
    }
  };
  let showPresetModal = false;
  let pendingPreset = null;

  // Reactive: which preset is currently active (inline check for proper reactivity)
  $: activePreset = Object.keys(KEY_PRESETS).find(key => {
    const preset = KEY_PRESETS[key];
    return (
      noteKeys.low.join(',') === preset.low.join(',') &&
      noteKeys.mid.join(',') === preset.mid.join(',') &&
      noteKeys.high.join(',') === preset.high.join(',')
    );
  }) || null;
  let isChangingPath = false;
  let customWindowKeywords = [];
  let newKeyword = "";
  let searchQuery = "";

  // Keybindings
  let keybindings = {
    pause_resume: "F9",
    stop: "F12",
    previous: "F10",
    next: "F11",
    mode_prev: "[",
    mode_next: "]",
    toggle_mini: "Insert"
  };
  let recordingKey = null; // Which key we're currently recording

  // Reactive keybinding labels for i18n
  $: keybindingLabels = {
    pause_resume: $t("settings.shortcuts.playPause"),
    stop: $t("settings.shortcuts.stop"),
    previous: $t("settings.shortcuts.previousTrack"),
    next: $t("settings.shortcuts.nextTrack"),
    mode_prev: $t("settings.shortcuts.modePrev"),
    mode_next: $t("settings.shortcuts.modeNext"),
    toggle_mini: $t("settings.shortcuts.miniMode")
  };

  // Settings sections for search/navigation (reactive for i18n)
  $: settingsSections = [
    { id: "keybindings", label: $t("settings.shortcuts.title"), icon: "mdi:keyboard-settings", keywords: ["keybindings", "shortcuts", "hotkeys", "keys", "bind"] },
    { id: "window", label: $t("settings.window.title"), icon: "mdi:application-outline", keywords: ["window", "detection", "process", "game"] },
    { id: "notemode", label: $t("noteMode.title"), icon: "mdi:music-note", keywords: ["note", "mode", "calculation", "mapping"] },
    { id: "keystyle", label: $t("settings.keyStyle.title"), icon: "mdi:piano", keywords: ["key", "style", "play", "21", "36"] },
    { id: "keyboard", label: $t("settings.keyboard.title"), icon: "mdi:piano", keywords: ["keyboard", "qwertz", "azerty", "layout", "keys", "notes"] },
    { id: "cloud", label: $t("settings.playback.cloudMode"), icon: "mdi:cloud", keywords: ["cloud", "gaming", "geforce", "input"] },
    { id: "storage", label: $t("settings.storage.title"), icon: "mdi:folder", keywords: ["storage", "album", "folder", "path"] },
    { id: "debug", label: $t("settings.debug.title"), icon: "mdi:bug", keywords: ["debug", "test", "keys", "spam"] },
  ];

  function scrollToSection(id) {
    const element = document.getElementById(`settings-${id}`);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
    searchQuery = "";
  }

  $: filteredSections = settingsSections
    .filter(s => s.id !== 'debug' || isDev) // Only show debug in dev mode
    .filter(s => searchQuery
      ? s.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.keywords.some(k => k.includes(searchQuery.toLowerCase()))
      : true
    );

  import { APP_VERSION, APP_FLAVOR } from "../version.js";
  let updateAvailable = null;

  const isDev = import.meta.env.DEV;

  // Key capture event listener cleanup
  let unlistenKeyCapture = null;

  // Re-enable keybindings when leaving settings (in case user was recording)
  onDestroy(() => {
    invoke('cmd_set_keybindings_enabled', { enabled: true }).catch(() => {});
    if (unlistenKeyCapture) unlistenKeyCapture();
  });

  onMount(async () => {
    // Listen for key capture events from backend (when app not focused)
    unlistenKeyCapture = await listen('key-captured', (event) => {
      const keyName = event.payload;

      // Handle escape for both recording modes
      if (keyName === 'Escape') {
        if (recordingKey) stopRecording();
        if (recordingNoteKey) stopRecordingNoteKey();
        return;
      }

      // Handle shortcut keybindings
      if (recordingKey) {
        applyKeyBinding(keyName);
        return;
      }

      // Handle note key bindings
      if (recordingNoteKey) {
        applyNoteKeyBinding(keyName);
        return;
      }
    });

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

    // Load note key bindings
    try {
      const keys = await invoke('get_note_keys');
      if (keys.low?.length === 7 && keys.mid?.length === 7 && keys.high?.length === 7) {
        noteKeys = keys;
      }
    } catch (e) {
      console.error("Failed to get note keys:", e);
    }

    // Load album path
    try {
      albumPath = await invoke('get_album_path');
    } catch (e) {
      console.error("Failed to get album path:", e);
    }

    // Load custom window keywords
    try {
      customWindowKeywords = await invoke('get_custom_window_keywords');
    } catch (e) {
      console.error("Failed to get custom window keywords:", e);
    }

    // Load keybindings
    try {
      keybindings = await invoke('cmd_get_keybindings');
    } catch (e) {
      console.error("Failed to get keybindings:", e);
    }

    // Check for updates
    checkForUpdates();
  });

  async function saveKeybindings() {
    try {
      await invoke('cmd_set_keybindings', { keybindings });
      // Notify App.svelte to reload keybindings
      window.dispatchEvent(new CustomEvent('keybindings-changed', { detail: keybindings }));
    } catch (e) {
      console.error("Failed to save keybindings:", e);
    }
  }

  async function resetKeybindings() {
    try {
      keybindings = await invoke('cmd_reset_keybindings');
      // Notify App.svelte to reload keybindings
      window.dispatchEvent(new CustomEvent('keybindings-changed', { detail: keybindings }));
    } catch (e) {
      console.error("Failed to reset keybindings:", e);
    }
  }

  async function startRecording(key) {
    await invoke('cmd_set_keybindings_enabled', { enabled: false });
    recordingKey = key;
    // Unfocus app so low-level hook can capture keys
    await invoke('cmd_unfocus_window');
  }

  async function stopRecording() {
    recordingKey = null;
    await invoke('cmd_set_keybindings_enabled', { enabled: true });
    // Refocus window
    const win = getCurrentWindow();
    await win.setFocus(true).catch(() => {});
  }

  // Apply key binding with smart swap
  function applyKeyBinding(keyName) {
    if (!recordingKey) return;

    const oldKey = keybindings[recordingKey];
    const conflictAction = Object.entries(keybindings).find(
      ([action, boundKey]) => boundKey === keyName && action !== recordingKey
    );

    if (conflictAction) {
      keybindings = {
        ...keybindings,
        [conflictAction[0]]: oldKey,
        [recordingKey]: keyName
      };
    } else {
      keybindings = {
        ...keybindings,
        [recordingKey]: keyName
      };
    }

    stopRecording();
    saveKeybindings();
  }

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

  // Note key binding functions
  async function saveNoteKeys() {
    try {
      await invoke('set_note_keys', noteKeys);
    } catch (e) {
      console.error("Failed to save note keys:", e);
    }
  }

  async function resetNoteKeys() {
    try {
      const keys = await invoke('reset_note_keys');
      noteKeys = keys;
    } catch (e) {
      console.error("Failed to reset note keys:", e);
    }
  }

  // Preset handling
  function selectPreset(presetKey) {
    pendingPreset = presetKey;
    showPresetModal = true;
  }

  async function applyPreset() {
    if (!pendingPreset || !KEY_PRESETS[pendingPreset]) return;

    const preset = KEY_PRESETS[pendingPreset];
    noteKeys = {
      low: [...preset.low],
      mid: [...preset.mid],
      high: [...preset.high]
    };
    await saveNoteKeys();
    showPresetModal = false;
    pendingPreset = null;
  }

  function cancelPreset() {
    showPresetModal = false;
    pendingPreset = null;
  }

  async function startRecordingNoteKey(octave, index) {
    await invoke('cmd_set_keybindings_enabled', { enabled: false });
    recordingNoteKey = { octave, index };
    await invoke('cmd_unfocus_window');
  }

  async function stopRecordingNoteKey() {
    recordingNoteKey = null;
    await invoke('cmd_set_keybindings_enabled', { enabled: true });
    const win = getCurrentWindow();
    await win.setFocus(true).catch(() => {});
  }

  function applyNoteKeyBinding(keyName) {
    if (!recordingNoteKey) return;

    // Allow: a-z, 0-9, and common punctuation (; , . /)
    const key = keyName.toLowerCase();
    const allowedKeys = /^[a-z0-9;,./]$/;

    // Also handle named keys
    const keyMap = {
      'semicolon': ';',
      'comma': ',',
      'period': '.',
      'slash': '/',
    };

    const mappedKey = keyMap[key] || key;

    if (!allowedKeys.test(mappedKey)) {
      stopRecordingNoteKey();
      return;
    }

    const { octave, index } = recordingNoteKey;
    noteKeys[octave][index] = mappedKey;
    noteKeys = { ...noteKeys }; // trigger reactivity

    stopRecordingNoteKey();
    saveNoteKeys();
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

  async function addWindowKeyword() {
    if (!newKeyword.trim()) return;
    const keyword = newKeyword.trim().toLowerCase();
    if (!customWindowKeywords.includes(keyword)) {
      customWindowKeywords = [...customWindowKeywords, keyword];
      await invoke('set_custom_window_keywords', { keywords: customWindowKeywords });
    }
    newKeyword = "";
  }

  async function removeWindowKeyword(keyword) {
    customWindowKeywords = customWindowKeywords.filter(k => k !== keyword);
    await invoke('set_custom_window_keywords', { keywords: customWindowKeywords });
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

  // Note calculation mode options (reactive for i18n)
  $: noteModesList = [
    { id: "Python", name: $t("noteMode.yuelyn"), description: $t("noteMode.yuelynDesc"), rmd21: true },
    { id: "Closest", name: $t("noteMode.closest"), description: $t("noteMode.closestDesc") },
    { id: "Wide", name: $t("noteMode.wide"), description: $t("noteMode.wideDesc") },
    { id: "Sharps", name: $t("noteMode.sharps"), description: $t("noteMode.sharpsDesc"), rmd36: true },
    { id: "Quantize", name: $t("noteMode.quantize"), description: $t("noteMode.quantizeDesc") },
    { id: "TransposeOnly", name: $t("noteMode.transposeOnly"), description: $t("noteMode.transposeOnlyDesc") },
    { id: "Pentatonic", name: $t("noteMode.pentatonic"), description: $t("noteMode.pentatonicDesc") },
    { id: "Chromatic", name: $t("noteMode.chromatic"), description: $t("noteMode.chromaticDesc") },
    { id: "Raw", name: $t("noteMode.raw"), description: $t("noteMode.rawDesc") },
  ];

  // Reactive: show RMD based on key mode
  $: noteModes = noteModesList.map(m => ({
    ...m,
    isRmd: ($keyMode === 'Keys36' && m.rmd36) || ($keyMode === 'Keys21' && m.rmd21)
  }));

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
  <div class="mb-4">
    <div class="flex items-center justify-between mb-3">
      <div>
        <h2 class="text-2xl font-bold">{$t("settings.title")}</h2>
        <p class="text-sm text-white/60">{$t("settings.subtitle")}</p>
      </div>
    </div>

    <!-- Search & Quick Nav -->
    <div class="relative mb-3">
      <Icon icon="mdi:magnify" class="absolute left-3 top-1/2 -translate-y-1/2 text-white/40 w-4 h-4" />
      <input
        type="text"
        placeholder={$t("settings.searchPlaceholder")}
        bind:value={searchQuery}
        class="w-full bg-white/5 border border-white/10 rounded-lg pl-9 pr-3 py-2 text-sm text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954]"
      />
    </div>

    <!-- Quick Navigation -->
    <div class="flex flex-wrap gap-1.5">
      {#each filteredSections as section}
        <button
          class="inline-flex items-center gap-1 px-2 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-xs text-white/60 hover:text-white transition-colors"
          onclick={() => scrollToSection(section.id)}
        >
          <Icon icon={section.icon} class="w-3 h-3" />
          {section.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Settings Sections -->
  <div
    bind:this={scrollContainer}
    onscroll={handleScroll}
    class="flex-1 overflow-y-auto space-y-6 pr-2 {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
  >
    <!-- Shortcuts Section -->
    <div
      id="settings-keybindings"
      class="bg-white/5 rounded-xl p-4 scroll-mt-4"
      in:fly={{ y: 10, duration: 200 }}
    >
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <Icon icon="mdi:keyboard-settings" class="w-5 h-5 text-[#1db954]" />
          <h3 class="text-lg font-semibold">{$t("settings.shortcuts.title")}</h3>
        </div>
        <button
          class="px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/15 text-white/60 hover:text-white text-xs font-medium transition-colors"
          onclick={resetKeybindings}
        >
          {$t("settings.shortcuts.resetToDefault")}
        </button>
      </div>

      <p class="text-sm text-white/60 mb-4">
        {$t("settings.shortcuts.description")}
      </p>

      <div class="grid grid-cols-2 gap-3">
        {#each Object.entries(keybindingLabels) as [key, label]}
          <div class="flex items-center justify-between bg-white/5 rounded-lg p-3">
            <span class="text-sm text-white/80">{label}</span>
            <button
              class="px-3 py-1.5 rounded-md font-mono text-sm min-w-[60px] text-center transition-all {recordingKey === key ? 'bg-[#1db954] text-black animate-pulse' : 'bg-white/10 hover:bg-white/20 text-white'}"
              onclick={() => startRecording(key)}
            >
              {recordingKey === key ? '...' : keybindings[key]}
            </button>
          </div>
        {/each}
      </div>

      <p class="text-xs text-white/40 mt-3">
        {$t("settings.shortcuts.supported")}
      </p>
    </div>

    <!-- Window Detection Section -->
    <div
      id="settings-window"
      class="bg-white/5 rounded-xl p-4 scroll-mt-4"
      in:fly={{ y: 10, duration: 200 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:application-outline" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">{$t("settings.window.title")}</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        {$t("settings.window.description")}
      </p>

      <!-- Add new keyword -->
      <div class="flex gap-2 mb-3">
        <input
          type="text"
          bind:value={newKeyword}
          placeholder={$t("settings.window.placeholder")}
          class="flex-1 px-3 py-2 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent"
          onkeydown={(e) => e.key === 'Enter' && addWindowKeyword()}
        />
        <button
          class="px-4 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors"
          onclick={addWindowKeyword}
        >
          {$t("settings.window.add")}
        </button>
      </div>

      <!-- Custom keywords -->
      {#if customWindowKeywords.length > 0}
        <div class="mb-3">
          <p class="text-xs text-white/40 mb-2">{$t("settings.window.custom")}</p>
          <div class="flex flex-wrap gap-1.5">
            {#each customWindowKeywords as keyword}
              <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/10 text-xs text-white/60">
                {keyword}
                <button
                  class="text-white/40 hover:text-red-400 transition-colors"
                  onclick={() => removeWindowKeyword(keyword)}
                  title="Remove"
                >
                  <Icon icon="mdi:close" class="w-3 h-3" />
                </button>
              </span>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Built-in keywords -->
      <div>
        <p class="text-xs text-white/40 mb-2">{$t("settings.window.builtIn")}</p>
        <div class="flex flex-wrap gap-1.5">
          {#each ['Where Winds Meet', 'WWM', 'GeForce Now', '燕云十六声', '연운'] as builtin}
            <span class="px-2 py-0.5 rounded-full bg-white/10 text-xs text-white/60">{builtin}</span>
          {/each}
        </div>
      </div>
    </div>

    <!-- Note Mode Section -->
    <div
      id="settings-notemode"
      class="bg-white/5 rounded-xl p-4 scroll-mt-4"
      in:fly={{ y: 10, duration: 200, delay: 50 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:music-note" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">{$t("settings.noteMode.title")}</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        {$t("settings.noteMode.description")}
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
              <div class="flex items-center gap-2">
                <span class="font-semibold text-white">{mode.name}</span>
                {#if mode.isRmd}
                  <span class="px-1.5 text-[10px] font-semibold bg-[#1db954]/20 text-[#1db954] rounded-full leading-4">RMD</span>
                {/if}
              </div>
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
      id="settings-keystyle"
      class="bg-white/5 rounded-xl p-4 scroll-mt-4"
      in:fly={{ y: 10, duration: 200, delay: 75 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:piano" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">{$t("settings.keyStyle.title")}</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        {$t("settings.keyStyle.description")}
      </p>

      <div class="flex gap-3">
        <button
          class="flex-1 p-4 rounded-lg border-2 transition-all duration-200 {$keyMode === 'Keys21'
            ? 'border-[#1db954] bg-[#1db954]/10'
            : 'border-white/10 bg-white/5 hover:border-white/20'}"
          onclick={() => setKeyMode('Keys21')}
        >
          <div class="text-center">
            <span class="text-2xl font-bold">{$t("keyMode.keys21")}</span>
            <p class="text-xs text-white/60 mt-1">{$t("keyMode.keys21Desc")}</p>
          </div>
        </button>
        <button
          class="flex-1 p-4 rounded-lg border-2 transition-all duration-200 {$keyMode === 'Keys36'
            ? 'border-[#1db954] bg-[#1db954]/10'
            : 'border-white/10 bg-white/5 hover:border-white/20'}"
          onclick={() => setKeyMode('Keys36')}
        >
          <div class="text-center">
            <span class="text-2xl font-bold">{$t("keyMode.keys36")}</span>
            <p class="text-xs text-white/60 mt-1">{$t("keyMode.keys36Desc")}</p>
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
          <span class="font-medium text-sm">{isTesting ? $t("settings.keyStyle.testing") : $t("settings.keyStyle.test21")}</span>
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
          <span class="font-medium text-sm">{isTesting36 ? $t("settings.keyStyle.testing") : $t("settings.keyStyle.test36")}</span>
        </button>
      </div>

      <!-- Spam Test (Dev Only) -->
      {#if isDev}
        <div id="settings-debug" class="mt-4 pt-4 border-t border-white/10 scroll-mt-4">
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

    <!-- Keyboard Layout / Note Keys -->
    <div
      id="settings-keyboard"
      class="bg-white/5 rounded-xl p-4 scroll-mt-4"
      in:fly={{ y: 10, duration: 200, delay: 100 }}
    >
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <Icon icon="mdi:keyboard" class="w-5 h-5 text-[#1db954]" />
          <h3 class="text-lg font-semibold">{$t("settings.keyboard.title")}</h3>
        </div>
        <button
          class="px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/15 text-white/60 hover:text-white text-xs font-medium transition-colors"
          onclick={resetNoteKeys}
        >
          {$t("settings.shortcuts.resetToDefault")}
        </button>
      </div>

      <p class="text-sm text-white/60 mb-3">
        {$t("settings.keyboard.description")}
      </p>

      {#if !activePreset}
        <div class="flex items-center gap-2 px-3 py-2 mb-3 rounded-lg bg-purple-500/10 border border-purple-500/20">
          <Icon icon="mdi:tune" class="w-4 h-4 text-purple-400" />
          <span class="text-xs text-purple-300">{$t("settings.keyboard.customLayout")}</span>
        </div>
      {/if}

      <!-- Preset Buttons -->
      <div class="flex gap-2 mb-4">
        {#each Object.entries(KEY_PRESETS) as [key, preset]}
          <button
            class="flex-1 py-2 px-3 rounded-lg transition-colors text-center relative {activePreset === key
              ? 'bg-[#1db954]/10 border-2 border-[#1db954]'
              : 'bg-white/5 hover:bg-white/10 border border-white/10 hover:border-white/20'}"
            onclick={() => selectPreset(key)}
          >
            {#if activePreset === key}
              <div class="absolute -top-1.5 -right-1.5 w-5 h-5 rounded-full bg-[#1db954] flex items-center justify-center">
                <Icon icon="mdi:check" class="w-3 h-3 text-white" />
              </div>
            {/if}
            <p class="font-medium text-sm {activePreset === key ? 'text-[#1db954]' : 'text-white'}">{preset.name}</p>
            <p class="text-xs {activePreset === key ? 'text-[#1db954]/60' : 'text-white/40'}">{preset.desc}</p>
          </button>
        {/each}
      </div>

      <!-- Note Key Grid -->
      <div class="space-y-3">
        <!-- High Octave -->
        <div class="bg-white/5 rounded-lg p-3">
          <p class="text-xs text-white/40 mb-2">{$t("settings.keyboard.highOctave")}</p>
          <div class="flex gap-1.5">
            {#each noteKeys.high as key, i}
              <button
                class="flex-1 py-2 rounded-md font-mono text-sm text-center transition-all uppercase {recordingNoteKey?.octave === 'high' && recordingNoteKey?.index === i ? 'bg-[#1db954] text-black animate-pulse' : 'bg-white/10 hover:bg-white/20 text-white'}"
                onclick={() => startRecordingNoteKey('high', i)}
              >
                {recordingNoteKey?.octave === 'high' && recordingNoteKey?.index === i ? '...' : key}
              </button>
            {/each}
          </div>
        </div>

        <!-- Mid Octave -->
        <div class="bg-white/5 rounded-lg p-3">
          <p class="text-xs text-white/40 mb-2">{$t("settings.keyboard.midOctave")}</p>
          <div class="flex gap-1.5">
            {#each noteKeys.mid as key, i}
              <button
                class="flex-1 py-2 rounded-md font-mono text-sm text-center transition-all uppercase {recordingNoteKey?.octave === 'mid' && recordingNoteKey?.index === i ? 'bg-[#1db954] text-black animate-pulse' : 'bg-white/10 hover:bg-white/20 text-white'}"
                onclick={() => startRecordingNoteKey('mid', i)}
              >
                {recordingNoteKey?.octave === 'mid' && recordingNoteKey?.index === i ? '...' : key}
              </button>
            {/each}
          </div>
        </div>

        <!-- Low Octave -->
        <div class="bg-white/5 rounded-lg p-3">
          <p class="text-xs text-white/40 mb-2">{$t("settings.keyboard.lowOctave")}</p>
          <div class="flex gap-1.5">
            {#each noteKeys.low as key, i}
              <button
                class="flex-1 py-2 rounded-md font-mono text-sm text-center transition-all uppercase {recordingNoteKey?.octave === 'low' && recordingNoteKey?.index === i ? 'bg-[#1db954] text-black animate-pulse' : 'bg-white/10 hover:bg-white/20 text-white'}"
                onclick={() => startRecordingNoteKey('low', i)}
              >
                {recordingNoteKey?.octave === 'low' && recordingNoteKey?.index === i ? '...' : key}
              </button>
            {/each}
          </div>
        </div>
      </div>

      <p class="text-xs text-white/40 mt-3">
        {$t("settings.keyboard.escapeToCancel")}
      </p>

      <!-- 36-Key Info -->
      <div class="mt-4 pt-4 border-t border-white/10">
        <p class="text-xs text-white/40 mb-2">{$t("settings.keyboard.keyMode36Info")}</p>
        <div class="space-y-1 text-xs">
          <div>
            <span class="text-orange-400">Shift+</span>
            <span class="text-white/60">{$t("settings.keyboard.shiftFor")}</span>
          </div>
          <div>
            <span class="text-blue-400">Ctrl+</span>
            <span class="text-white/60">{$t("settings.keyboard.ctrlFor")}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Playback Settings Section -->
    <div
      id="settings-playback"
      class="bg-white/5 rounded-xl p-4 scroll-mt-4"
      in:fly={{ y: 10, duration: 200, delay: 150 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:play-circle-outline" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">{$t("settings.playback.title")}</h3>
      </div>

      <!-- Smart Pause Toggle -->
      <div class="flex items-center justify-between py-3">
        <div>
          <p class="font-medium text-white">{$t("settings.playback.smartPause")}</p>
          <p class="text-sm text-white/60">{$t("settings.playback.smartPauseDesc")}</p>
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
      <div id="settings-cloud" class="flex items-center justify-between py-3 border-t border-white/10 scroll-mt-4">
        <div>
          <p class="font-medium text-white">{$t("settings.playback.cloudMode")}</p>
          <p class="text-sm text-white/60">{$t("settings.playback.cloudModeDesc")}</p>
          {#if cloudMode}
            <div class="text-xs text-orange-400 mt-1 space-y-0.5">
              <p>⚠️ {$t("settings.playback.cloudWarning1")}</p>
              <p>⚠️ {$t("settings.playback.cloudWarning2")}</p>
              <p>⚠️ {$t("settings.playback.cloudWarning3")}</p>
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
      id="settings-storage"
      class="bg-white/5 rounded-xl p-4 scroll-mt-4"
      in:fly={{ y: 10, duration: 200, delay: 175 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:folder-music" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">{$t("settings.storage.title")}</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        {$t("settings.storage.description")}
      </p>

      <!-- Current Path Display -->
      <div class="bg-white/5 rounded-lg p-3 mb-4">
        <p class="text-xs text-white/40 mb-1">{$t("settings.storage.currentFolder")}</p>
        <p class="text-sm text-white font-mono truncate" title={albumPath}>
          {albumPath || $t("settings.storage.loading")}
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
          <span class="font-medium text-sm">{isChangingPath ? $t("settings.storage.selecting") : $t("settings.storage.browse")}</span>
        </button>
        <button
          class="py-3 px-4 rounded-lg bg-white/10 hover:bg-white/15 transition-colors flex items-center justify-center gap-2"
          onclick={resetAlbumPath}
          title={$t("settings.storage.resetToDefault")}
        >
          <Icon icon="mdi:restore" class="w-5 h-5" />
          <span class="font-medium text-sm">{$t("settings.storage.reset")}</span>
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
        <h3 class="text-lg font-semibold">{$t("settings.about.title")}</h3>
      </div>

      <div class="text-sm text-white/60 space-y-3">
        <p>{$t("settings.about.description")}</p>
        <div class="flex items-center gap-2">
          <span class="text-xs text-white/40">{$t("settings.about.byYueLyn")}</span>
          <span class="text-white/20">•</span>
          <span class="text-xs text-white/40">v{APP_VERSION}{APP_FLAVOR ? `(${APP_FLAVOR})` : ''}</span>
        </div>

        {#if updateAvailable}
          <button
            class="w-full flex items-center gap-3 p-3 rounded-lg bg-[#1db954]/10 hover:bg-[#1db954]/20 transition-colors"
            onclick={() => window.dispatchEvent(new CustomEvent('open-update-modal'))}
          >
            <Icon icon="mdi:download-circle" class="w-6 h-6 text-[#1db954]" />
            <div class="text-left">
              <p class="text-sm font-medium text-[#1db954]">{$t("settings.about.updateAvailable")}</p>
              <p class="text-xs text-white/50">v{updateAvailable.version} - {$t("settings.about.clickToDownload")}</p>
            </div>
          </button>
        {:else}
          <div class="flex items-center gap-2 text-xs text-white/40">
            <Icon icon="mdi:check-circle" class="w-4 h-4 text-[#1db954]" />
            <span>{$t("settings.about.latestVersion")}</span>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<!-- Preset Confirmation Modal -->
{#if showPresetModal && pendingPreset && KEY_PRESETS[pendingPreset]}
  <div
    class="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center"
    onclick={cancelPreset}
    in:fade={{ duration: 150 }}
  >
    <div
      class="bg-[#282828] rounded-xl p-6 max-w-md mx-4 shadow-2xl border border-white/10"
      onclick={(e) => e.stopPropagation()}
      in:fly={{ y: 20, duration: 200 }}
    >
      <div class="flex items-center gap-3 mb-4">
        <div class="w-10 h-10 rounded-full bg-orange-500/20 flex items-center justify-center">
          <Icon icon="mdi:alert" class="w-5 h-5 text-orange-400" />
        </div>
        <div>
          <h3 class="text-lg font-semibold text-white">{$t("settings.presets.applyLayout", { values: { name: KEY_PRESETS[pendingPreset].name } })}</h3>
          <p class="text-sm text-white/60">{KEY_PRESETS[pendingPreset].desc}</p>
        </div>
      </div>

      <div class="bg-white/5 rounded-lg p-3 mb-4">
        <p class="text-xs text-white/40 mb-2">{$t("settings.presets.willSetKeysTo")}</p>
        <div class="space-y-1 text-sm font-mono">
          <p><span class="text-white/40">{$t("settings.presets.high")}</span> <span class="text-white uppercase">{KEY_PRESETS[pendingPreset].high.join(' ')}</span></p>
          <p><span class="text-white/40">{$t("settings.presets.mid")}</span> <span class="text-white uppercase">{KEY_PRESETS[pendingPreset].mid.join(' ')}</span></p>
          <p><span class="text-white/40">{$t("settings.presets.low")}</span> <span class="text-white uppercase">{KEY_PRESETS[pendingPreset].low.join(' ')}</span></p>
        </div>
      </div>

      <div class="flex items-center gap-2 p-3 rounded-lg bg-orange-500/10 border border-orange-500/20 mb-4">
        <Icon icon="mdi:information" class="w-4 h-4 text-orange-400 flex-shrink-0" />
        <p class="text-xs text-orange-300">{$t("settings.presets.overrideWarning")}</p>
      </div>

      <div class="flex gap-3">
        <button
          class="flex-1 py-2.5 px-4 rounded-lg bg-white/10 hover:bg-white/15 text-white font-medium transition-colors"
          onclick={cancelPreset}
        >
          {$t("common.cancel")}
        </button>
        <button
          class="flex-1 py-2.5 px-4 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium transition-colors"
          onclick={applyPreset}
        >
          {$t("common.apply")}
        </button>
      </div>
    </div>
  </div>
{/if}
