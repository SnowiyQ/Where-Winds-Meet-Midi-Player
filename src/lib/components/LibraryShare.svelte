<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { onMount } from "svelte";
  import {
    libraryEnabled,
    libraryConnected,
    globalSongs,
    onlinePeers,
    downloadProgress,
    libraryError,
    connectLibrary,
    disconnectLibrary,
    toggleLibrary,
    requestSong,
    shareAll,
    toggleShareAll,
    sharedSongs,
    setSharedSongs,
    initLibrary,
    refreshSongs,
    developerMode,
    toggleDeveloperMode,
    discoveryServerUrl,
    setDiscoveryServer,
    startServer,
    isHostingServer,
  } from "../stores/library.js";
  import { midiFiles } from "../stores/player.js";

  let searchQuery = "";
  let showSharePicker = false;
  let shareSearchQuery = "";
  let showDevSettings = false;
  let serverUrlInput = "";
  let serverPort = 3456;
  let showDownloadModal = false;
  let downloadingSong = null;

  // Computed shared songs count
  $: sharedCount = $shareAll ? $midiFiles.length : $sharedSongs.length;

  // Filtered songs for share picker
  $: sharePickerSongs = $midiFiles.filter(f =>
    f.name.toLowerCase().includes(shareSearchQuery.toLowerCase())
  );

  // Filter global songs by search
  $: filteredSongs = searchQuery
    ? $globalSongs.filter(song => song.name?.toLowerCase().includes(searchQuery.toLowerCase()))
    : $globalSongs;

  $: console.log('[UI] globalSongs:', $globalSongs.length, 'filteredSongs:', filteredSongs.length);

  function toggleSongShare(path) {
    const current = $sharedSongs;
    if (current.includes(path)) {
      setSharedSongs(current.filter(p => p !== path));
    } else {
      setSharedSongs([...current, path]);
    }
  }

  function selectAllSongs() {
    setSharedSongs($midiFiles.map(f => f.path));
  }

  function deselectAllSongs() {
    setSharedSongs([]);
  }

  // Scroll mask
  let scrollContainer;
  let showTopMask = false;
  let showBottomMask = false;

  function handleScroll(e) {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    showTopMask = scrollTop > 10;
    showBottomMask = scrollTop + clientHeight < scrollHeight - 10;
  }

  onMount(() => {
    initLibrary();
    serverUrlInput = $discoveryServerUrl;
    setTimeout(() => {
      if (scrollContainer) {
        const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
        showBottomMask = scrollHeight > clientHeight;
      }
    }, 100);
  });

  // Reactive set of owned song hashes (updates when midiFiles changes)
  $: ownedHashes = new Set($midiFiles.map(f => f.hash));

  function openDownloadModal(song) {
    downloadingSong = song;
    showDownloadModal = true;
  }

  async function confirmDownload() {
    if (!downloadingSong) return;
    showDownloadModal = false;
    await requestSong(downloadingSong.peerId, downloadingSong.hash, downloadingSong.name);
    downloadingSong = null;
  }

  function formatDuration(seconds) {
    if (!seconds) return "--:--";
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }

  function saveServerUrl() {
    setDiscoveryServer(serverUrlInput);
  }

  async function handleStartServer() {
    await startServer(serverPort);
  }
</script>

<div class="h-full flex flex-col min-h-0 -mx-1">
  <div
    bind:this={scrollContainer}
    onscroll={handleScroll}
    class="flex-1 overflow-y-auto scrollbar-thin min-h-0 px-1 pb-2 {showTopMask && showBottomMask ? 'scroll-mask-both' : showTopMask ? 'scroll-mask-top' : showBottomMask ? 'scroll-mask-bottom' : ''}"
  >
    <!-- Header -->
    <div class="mb-4">
      <div class="flex items-center gap-3 mb-2">
        <div class="w-12 h-12 rounded-lg bg-white/10 flex items-center justify-center">
          <Icon icon="mdi:earth" class="w-6 h-6 text-[#1db954]" />
        </div>
        <div class="flex-1">
          <h2 class="text-xl font-bold">Song Library</h2>
          <p class="text-xs text-white/60">
            {#if $libraryConnected}
              {$onlinePeers} peer{$onlinePeers !== 1 ? 's' : ''} online &bull; {$globalSongs.length} songs
            {:else}
              Browse & share songs globally
            {/if}
          </p>
        </div>
        <!-- Toggle -->
        <button
          class="w-10 h-5 rounded-full transition-colors {$libraryEnabled ? 'bg-[#1db954]' : 'bg-white/20'}"
          onclick={toggleLibrary}
          title={$libraryEnabled ? "Disable sharing" : "Enable sharing"}
        >
          <div class="w-4 h-4 rounded-full bg-white transition-transform {$libraryEnabled ? 'translate-x-5' : 'translate-x-0.5'}"></div>
        </button>
      </div>
    </div>

    {#if !$libraryEnabled}
      <!-- Disabled State -->
      <div class="flex flex-col items-center justify-center py-12 text-center" in:fade>
        <div class="w-16 h-16 rounded-full bg-white/5 flex items-center justify-center mb-4">
          <Icon icon="mdi:share-off" class="w-8 h-8 text-white/30" />
        </div>
        <p class="text-white/60 mb-2">Library sharing is disabled</p>
        <p class="text-xs text-white/40 mb-4">Enable to browse and share songs with others</p>
        <button
          class="px-4 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors"
          onclick={toggleLibrary}
        >
          Enable Sharing
        </button>
      </div>
    {:else if !$libraryConnected}
      <!-- Connecting State -->
      <div class="space-y-4" in:fade>
        <!-- Developer Settings (available while connecting) -->
        <div class="p-3 rounded-lg bg-white/5">
          <button
            class="w-full flex items-center justify-between text-xs text-white/70 hover:text-white transition-colors"
            onclick={() => showDevSettings = !showDevSettings}
          >
            <span class="flex items-center gap-2">
              <Icon icon="mdi:developer-board" class="w-3.5 h-3.5" />
              Developer Settings
            </span>
            <Icon icon={showDevSettings ? "mdi:chevron-up" : "mdi:chevron-down"} class="w-4 h-4" />
          </button>

          {#if showDevSettings}
            <div class="mt-3 pt-3 border-t border-white/10 space-y-3" transition:fade={{ duration: 150 }}>
              <!-- Discovery Server URL -->
              <div class="space-y-1">
                <label class="text-xs text-white/50">Discovery Server URL</label>
                <div class="flex gap-2">
                  <input
                    type="text"
                    bind:value={serverUrlInput}
                    placeholder="https://discovery.example.com"
                    class="flex-1 bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-xs text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954]"
                  />
                  <button
                    class="px-3 py-1.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white text-xs font-medium transition-colors"
                    onclick={saveServerUrl}
                  >
                    Save
                  </button>
                  <button
                    class="px-2 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white/70 hover:text-white text-xs transition-colors"
                    onclick={() => { serverUrlInput = 'https://discovery.chuaii.me'; saveServerUrl(); }}
                    title="Reset to default"
                  >
                    <Icon icon="mdi:refresh" class="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              <!-- Host Server -->
              <div class="space-y-1">
                <label class="text-xs text-white/50">Host Discovery Server</label>
                <div class="flex gap-2 items-center">
                  <input
                    type="number"
                    bind:value={serverPort}
                    placeholder="3456"
                    class="w-20 bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-xs text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954]"
                  />
                  <button
                    class="px-3 py-1.5 rounded-lg text-xs font-medium transition-colors {$isHostingServer ? 'bg-green-500/20 text-green-400' : 'bg-white/10 hover:bg-white/20 text-white'}"
                    onclick={handleStartServer}
                    disabled={$isHostingServer}
                  >
                    {$isHostingServer ? 'Server Running' : 'Start Server'}
                  </button>
                </div>
                <p class="text-xs text-white/40">Run a discovery server on this machine</p>
              </div>
            </div>
          {/if}
        </div>

        <!-- Connecting spinner -->
        <div class="flex flex-col items-center justify-center py-8 text-center">
          <Icon icon="mdi:loading" class="w-12 h-12 text-[#1db954] animate-spin mb-4" />
          <p class="text-white/60">Connecting to network...</p>
        </div>

        <!-- Error Display -->
        {#if $libraryError}
          <div class="p-3 rounded-lg bg-red-500/10 border border-red-500/30 flex items-center gap-2" transition:fade>
            <Icon icon="mdi:alert-circle" class="w-4 h-4 text-red-400" />
            <p class="text-xs text-red-400 flex-1">{$libraryError}</p>
            <button
              class="text-xs text-red-400 hover:text-red-300"
              onclick={() => libraryError.set(null)}
            >
              Dismiss
            </button>
          </div>
        {/if}
      </div>
    {:else}
      <!-- Connected State -->
      <div class="space-y-4" in:fade>
        <!-- Connection Info -->
        <div class="p-3 rounded-lg bg-white/5 space-y-2">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <div class="w-2 h-2 rounded-full bg-[#1db954] animate-pulse"></div>
              <span class="text-xs text-white/70">Connected</span>
              <span class="text-xs text-white/40">•</span>
              <span class="text-xs text-[#1db954]">Sharing {sharedCount} song{sharedCount !== 1 ? 's' : ''}</span>
            </div>
            <button
              class="text-xs text-white/50 hover:text-white transition-colors flex items-center gap-1"
              onclick={refreshSongs}
              title="Refresh song list"
            >
              <Icon icon="mdi:refresh" class="w-3 h-3" />
              Refresh
            </button>
          </div>

          <!-- Sharing Options -->
          <div class="pt-2 border-t border-white/10 space-y-2">
            <!-- Share All Toggle -->
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Icon icon="mdi:share-variant" class="w-3.5 h-3.5 text-white/50" />
                <span class="text-xs text-white/70">Share all songs</span>
              </div>
              <button
                class="w-8 h-4 rounded-full transition-colors {$shareAll ? 'bg-[#1db954]' : 'bg-white/20'}"
                onclick={toggleShareAll}
              >
                <div class="w-3 h-3 rounded-full bg-white transition-transform {$shareAll ? 'translate-x-4' : 'translate-x-0.5'}"></div>
              </button>
            </div>

            <!-- Select Songs Button (when not sharing all) -->
            {#if !$shareAll}
              <button
                class="w-full py-1.5 px-3 rounded-lg bg-white/5 hover:bg-white/10 text-xs text-white/70 hover:text-white transition-colors flex items-center justify-between"
                onclick={() => showSharePicker = true}
              >
                <span class="flex items-center gap-2">
                  <Icon icon="mdi:playlist-check" class="w-3.5 h-3.5" />
                  Select songs to share
                </span>
                <span class="text-white/50">{sharedCount} selected</span>
              </button>
            {/if}
          </div>
        </div>

        <!-- Developer Settings -->
        <div class="p-3 rounded-lg bg-white/5">
          <button
            class="w-full flex items-center justify-between text-xs text-white/70 hover:text-white transition-colors"
            onclick={() => showDevSettings = !showDevSettings}
          >
            <span class="flex items-center gap-2">
              <Icon icon="mdi:developer-board" class="w-3.5 h-3.5" />
              Developer Settings
            </span>
            <Icon icon={showDevSettings ? "mdi:chevron-up" : "mdi:chevron-down"} class="w-4 h-4" />
          </button>

          {#if showDevSettings}
            <div class="mt-3 pt-3 border-t border-white/10 space-y-3" transition:fade={{ duration: 150 }}>
              <!-- Discovery Server URL -->
              <div class="space-y-1">
                <label class="text-xs text-white/50">Discovery Server URL</label>
                <div class="flex gap-2">
                  <input
                    type="text"
                    bind:value={serverUrlInput}
                    placeholder="https://discovery.example.com"
                    class="flex-1 bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-xs text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954]"
                  />
                  <button
                    class="px-3 py-1.5 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white text-xs font-medium transition-colors"
                    onclick={saveServerUrl}
                  >
                    Save
                  </button>
                  <button
                    class="px-2 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white/70 hover:text-white text-xs transition-colors"
                    onclick={() => { serverUrlInput = 'https://discovery.chuaii.me'; saveServerUrl(); }}
                    title="Reset to default"
                  >
                    <Icon icon="mdi:refresh" class="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              <!-- Host Server -->
              <div class="space-y-1">
                <label class="text-xs text-white/50">Host Discovery Server</label>
                <div class="flex gap-2 items-center">
                  <input
                    type="number"
                    bind:value={serverPort}
                    placeholder="3456"
                    class="w-20 bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-xs text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954]"
                  />
                  <button
                    class="px-3 py-1.5 rounded-lg text-xs font-medium transition-colors {$isHostingServer ? 'bg-green-500/20 text-green-400' : 'bg-white/10 hover:bg-white/20 text-white'}"
                    onclick={handleStartServer}
                    disabled={$isHostingServer}
                  >
                    {$isHostingServer ? 'Server Running' : 'Start Server'}
                  </button>
                </div>
                <p class="text-xs text-white/40">Run a discovery server on this machine</p>
              </div>
            </div>
          {/if}
        </div>

        <!-- Download Progress (at top) -->
        {#if $downloadProgress}
          <div class="p-3 rounded-lg bg-[#1db954]/10 border border-[#1db954]/30 flex items-center gap-3" transition:fade>
            <Icon icon="mdi:loading" class="w-5 h-5 text-[#1db954] animate-spin flex-shrink-0" />
            <div class="flex-1 min-w-0">
              <p class="text-sm font-medium text-white truncate">{$downloadProgress.songName}</p>
              <p class="text-xs text-[#1db954]">{$downloadProgress.status}</p>
            </div>
            <div class="w-12 text-right">
              <span class="text-xs text-[#1db954] font-medium">{$downloadProgress.progress}%</span>
            </div>
          </div>
        {/if}

        <!-- Error Display (at top) -->
        {#if $libraryError}
          <div class="p-3 rounded-lg bg-red-500/10 border border-red-500/30 flex items-center gap-2" transition:fade>
            <Icon icon="mdi:alert-circle" class="w-4 h-4 text-red-400 flex-shrink-0" />
            <p class="text-xs text-red-400 flex-1">{$libraryError}</p>
            <button
              class="text-xs text-red-400 hover:text-red-300"
              onclick={() => libraryError.set(null)}
            >
              Dismiss
            </button>
          </div>
        {/if}

        <!-- Search -->
        {#if $globalSongs.length > 0}
          <div class="relative">
            <Icon icon="mdi:magnify" class="absolute left-3 top-1/2 -translate-y-1/2 text-white/40 w-4 h-4" />
            <input
              type="text"
              placeholder="Search {$globalSongs.length} available songs..."
              bind:value={searchQuery}
              class="w-full bg-white/5 border border-white/10 rounded-lg pl-9 pr-3 py-2 text-sm text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954]"
            />
          </div>
        {/if}

        <!-- Song List -->
        {#if filteredSongs.length > 0}
          <div class="space-y-1">
            {#each filteredSongs as song, i (song.hash + song.peerId + i)}
              {@const hasIt = ownedHashes.has(song.hash)}
              <div
                class="group flex items-center gap-3 p-2 rounded-lg hover:bg-white/5 transition-colors"
                in:fly={{ y: 10, duration: 150 }}
              >
                <!-- Song Icon -->
                <div class="w-8 h-8 rounded bg-white/10 flex items-center justify-center flex-shrink-0">
                  <Icon icon="mdi:music-note" class="w-4 h-4 text-white/50" />
                </div>

                <!-- Song Info -->
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-medium truncate {hasIt ? 'text-white/50' : 'text-white'}">
                    {song.name}
                  </p>
                  <p class="text-xs text-white/40">
                    {song.peerName} &bull; {song.bpm || '?'} BPM &bull; {formatDuration(song.duration)}
                  </p>
                </div>

                <!-- Download Button -->
                <div class="flex-shrink-0">
                  {#if hasIt}
                    <span class="text-xs text-[#1db954] flex items-center gap-1">
                      <Icon icon="mdi:check" class="w-4 h-4" />
                      Owned
                    </span>
                  {:else if $downloadProgress?.songName === song.name}
                    <div class="flex items-center gap-2">
                      <Icon icon="mdi:loading" class="w-4 h-4 text-[#1db954] animate-spin" />
                      <span class="text-xs text-white/50">{$downloadProgress.status}</span>
                    </div>
                  {:else}
                    <button
                      class="p-1.5 rounded-full text-white/30 hover:text-[#1db954] hover:bg-white/10 transition-all opacity-0 group-hover:opacity-100"
                      onclick={() => openDownloadModal(song)}
                      title="Download"
                    >
                      <Icon icon="mdi:download" class="w-5 h-5" />
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {:else if $globalSongs.length === 0}
          <div class="p-4 rounded-lg bg-white/5 text-center">
            <Icon icon="mdi:music-note-off" class="w-8 h-8 text-white/30 mx-auto mb-2" />
            <p class="text-xs text-white/50">No songs from other users yet</p>
            <p class="text-xs text-white/40 mt-1">Your {sharedCount} song{sharedCount !== 1 ? 's are' : ' is'} visible to others</p>
          </div>
        {:else}
          <div class="p-4 rounded-lg bg-white/5 text-center">
            <Icon icon="mdi:magnify" class="w-8 h-8 text-white/30 mx-auto mb-2" />
            <p class="text-xs text-white/50">No songs match "{searchQuery}"</p>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<!-- Share Picker Modal -->
{#if showSharePicker}
  <div class="fixed inset-0 z-50 flex items-center justify-center" transition:fade={{ duration: 150 }}>
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { showSharePicker = false; shareSearchQuery = ""; }}
    ></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[400px] max-w-[90vw] max-h-[80vh] overflow-hidden flex flex-col"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-white/10">
        <div>
          <h3 class="text-lg font-bold">Select Songs to Share</h3>
          <p class="text-xs text-white/50">{$sharedSongs.length} of {$midiFiles.length} selected</p>
        </div>
        <button
          class="p-1 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors"
          onclick={() => { showSharePicker = false; shareSearchQuery = ""; }}
        >
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <!-- Search & Actions -->
      <div class="p-3 border-b border-white/10 space-y-2">
        <div class="relative">
          <Icon icon="mdi:magnify" class="absolute left-3 top-1/2 -translate-y-1/2 text-white/40 w-4 h-4" />
          <input
            type="text"
            placeholder="Search songs..."
            bind:value={shareSearchQuery}
            class="w-full bg-white/5 border border-white/10 rounded-lg pl-9 pr-3 py-2 text-sm text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954]"
          />
        </div>
        <div class="flex gap-2">
          <button
            class="flex-1 py-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-xs text-white/70 hover:text-white transition-colors"
            onclick={selectAllSongs}
          >
            Select All
          </button>
          <button
            class="flex-1 py-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-xs text-white/70 hover:text-white transition-colors"
            onclick={deselectAllSongs}
          >
            Deselect All
          </button>
        </div>
      </div>

      <!-- Song List -->
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        {#each sharePickerSongs as file (file.path)}
          {@const isSelected = $sharedSongs.includes(file.path)}
          <button
            class="w-full flex items-center gap-3 p-2 rounded-lg transition-colors {isSelected ? 'bg-[#1db954]/20' : 'hover:bg-white/5'}"
            onclick={() => toggleSongShare(file.path)}
          >
            <!-- Checkbox -->
            <div class="w-5 h-5 rounded border-2 flex items-center justify-center flex-shrink-0 transition-colors {isSelected ? 'bg-[#1db954] border-[#1db954]' : 'border-white/30'}">
              {#if isSelected}
                <Icon icon="mdi:check" class="w-3.5 h-3.5 text-white" />
              {/if}
            </div>

            <!-- Song Info -->
            <div class="flex-1 min-w-0 text-left">
              <p class="text-sm font-medium truncate {isSelected ? 'text-[#1db954]' : 'text-white'}">{file.name}</p>
              <p class="text-xs text-white/40">{file.bpm || '?'} BPM</p>
            </div>
          </button>
        {/each}

        {#if sharePickerSongs.length === 0}
          <div class="text-center py-8 text-white/40">
            <Icon icon="mdi:music-note-off" class="w-8 h-8 mx-auto mb-2 opacity-50" />
            <p class="text-sm">No songs found</p>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="p-3 border-t border-white/10">
        <button
          class="w-full py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors"
          onclick={() => { showSharePicker = false; shareSearchQuery = ""; }}
        >
          Done
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Download Confirmation Modal -->
{#if showDownloadModal && downloadingSong}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { showDownloadModal = false; downloadingSong = null; }}
    ></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[360px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <div class="p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-[#1db954]/20 flex items-center justify-center mx-auto mb-3">
          <Icon icon="mdi:download" class="w-6 h-6 text-[#1db954]" />
        </div>
        <h3 class="text-lg font-bold mb-2">Download Song?</h3>
        <p class="text-sm text-white/60 mb-1">"{downloadingSong.name}"</p>
        <p class="text-xs text-white/40">from {downloadingSong.peerName}</p>
      </div>

      <div class="flex gap-2 p-4 pt-0">
        <button
          class="flex-1 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition-colors"
          onclick={() => { showDownloadModal = false; downloadingSong = null; }}
        >
          Cancel
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors"
          onclick={confirmDownload}
        >
          Download
        </button>
      </div>
    </div>
  </div>
{/if}
