<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import {
    bandEnabled,
    isHost,
    roomCode,
    connectedPeers,
    myTrackId,
    mySlot,
    availableTracks,
    bandStatus,
    bandPlayMode,
    createRoom,
    joinRoom,
    leaveRoom,
    assignTrack,
    assignSlot,
    autoAssignSlots,
    setBandPlayMode,
    bandPlay,
    bandPause,
    bandStop,
    loadTracksFromFile,
    bandSelectedSong,
    startBandSongSelect,
    myReady,
    toggleReady,
    allMembersReady,
    bandFilePath,
    hostDelay,
    kickPlayer,
    autoReady,
    isCalibrating,
    startCalibration,
    stopCalibration,
    useTurnServer,
  } from "../stores/band.js";
  import { midiFiles, isPlaying, isPaused } from "../stores/player.js";
  import { createEventDispatcher, onMount } from "svelte";

  const dispatch = createEventDispatcher();

  let joinCode = "";
  let playerName = "Player";
  let isStartingPlayback = false;

  // Load saved player name
  const PLAYER_NAME_KEY = "wwm-band-player-name";
  let isCreating = false;
  let isJoining = false;
  let error = null;
  let isLoadingTracks = false;
  let copied = false;

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
    // Load saved player name
    const savedName = localStorage.getItem(PLAYER_NAME_KEY);
    if (savedName) {
      playerName = savedName;
    }

    setTimeout(() => {
      if (scrollContainer) {
        const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
        showBottomMask = scrollHeight > clientHeight;
      }
    }, 100);
  });

  // Save player name when changed
  function savePlayerName() {
    if (playerName.trim()) {
      localStorage.setItem(PLAYER_NAME_KEY, playerName.trim());
    }
  }

  async function handleCreateRoom() {
    isCreating = true;
    error = null;
    const name = playerName || "Host";
    savePlayerName();
    try {
      await createRoom(name);
    } catch (e) {
      error = e.message || "Failed to create room";
    }
    isCreating = false;
  }

  async function handleJoinRoom() {
    if (!joinCode.trim()) {
      error = "Enter a room code";
      return;
    }
    isJoining = true;
    error = null;
    const name = playerName || "Player";
    savePlayerName();
    try {
      await joinRoom(joinCode, name);
    } catch (e) {
      error = e.message || "Failed to join room";
    }
    isJoining = false;
  }

  function handleLeave() {
    leaveRoom();
    joinCode = "";
    error = null;
  }

  function copyRoomCode() {
    navigator.clipboard.writeText($roomCode);
    copied = true;
    setTimeout(() => copied = false, 2000);
  }

  function getLatencyColor(latency) {
    if (latency < 50) return "text-green-400";
    if (latency < 100) return "text-yellow-400";
    return "text-red-400";
  }

  async function handleLoadTracks() {
    if (!$bandSelectedSong?.path || !$isHost) return;
    isLoadingTracks = true;
    await loadTracksFromFile($bandSelectedSong.path);
    isLoadingTracks = false;
  }

  function handleSelectSong() {
    startBandSongSelect();
    dispatch('selectsong');
  }

  function getAssignedPlayer(trackId) {
    return $connectedPeers.find(p => p.trackId === trackId);
  }

  function handleTrackAssign(trackId, peerId) {
    if ($isHost) {
      assignTrack(peerId, trackId);
    }
  }

  function assignTrackToPlayer(peerId, trackId) {
    if ($isHost) {
      // trackId comes as string from select, convert to number or null
      const tid = trackId === '' ? null : parseInt(trackId);
      assignTrack(peerId, tid);
    }
  }

  function autoAssignTracks() {
    if (!$isHost || $availableTracks.length === 0) return;

    const peers = $connectedPeers;
    const tracks = $availableTracks;

    // Assign tracks to players in order
    peers.forEach((peer, index) => {
      if (index < tracks.length) {
        assignTrack(peer.id, tracks[index].id);
      } else {
        // More players than tracks - assign null
        assignTrack(peer.id, null);
      }
    });
  }

  function clearAllAssignments() {
    if (!$isHost) return;
    $connectedPeers.forEach(peer => {
      assignTrack(peer.id, null);
    });
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
          <Icon icon="mdi:account-group" class="w-6 h-6 text-[#1db954]" />
        </div>
        <div>
          <h2 class="text-xl font-bold">Band Mode</h2>
          <p class="text-xs text-white/60">
            {#if $bandStatus === 'connected'}
              {$connectedPeers.length} player{$connectedPeers.length !== 1 ? 's' : ''}
            {:else}
              Play with friends
            {/if}
          </p>
        </div>
      </div>
    </div>

    {#if $bandStatus === 'disconnected'}
      <!-- Not Connected -->
      <div class="space-y-3" in:fade>
        <!-- Player Name -->
        <div>
          <label class="block text-xs text-white/50 mb-1">Your Name</label>
          <input
            type="text"
            bind:value={playerName}
            placeholder="Enter your name"
            class="w-full px-3 py-1.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954] focus:border-transparent"
            onblur={savePlayerName}
            onchange={savePlayerName}
          />
        </div>

        <!-- Create Room -->
        <div>
          <label class="block text-xs text-white/50 mb-1">Host a Session</label>
          <button
            class="w-full py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
            onclick={handleCreateRoom}
            disabled={isCreating}
          >
            {#if isCreating}
              <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
            {:else}
              <Icon icon="mdi:plus" class="w-4 h-4" />
            {/if}
            Create Room
          </button>
        </div>

        <!-- Join Room -->
        <div>
          <label class="block text-xs text-white/50 mb-1">Join a Session</label>
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={joinCode}
              placeholder="CODE"
              maxlength="6"
              class="flex-1 px-3 py-1.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-[#1db954] uppercase tracking-widest text-center font-mono"
              onkeydown={(e) => e.key === 'Enter' && handleJoinRoom()}
            />
            <button
              class="px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition-colors disabled:opacity-50"
              onclick={handleJoinRoom}
              disabled={isJoining}
            >
              {#if isJoining}
                <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
              {:else}
                Join
              {/if}
            </button>
          </div>
        </div>

        <!-- TURN Server Toggle -->
        <div class="flex items-center justify-between py-2 px-3 bg-white/5 rounded-lg">
          <div class="flex items-center gap-2">
            <Icon icon="mdi:server-network" class="w-4 h-4 text-white/40" />
            <div>
              <p class="text-xs text-white/80">Use Relay Server</p>
              <p class="text-[10px] text-white/40">Enable if direct connection fails</p>
            </div>
          </div>
          <button
            class="relative w-9 h-5 rounded-full transition-colors duration-200 {$useTurnServer ? 'bg-[#1db954]' : 'bg-white/20'}"
            onclick={() => useTurnServer.update(v => !v)}
          >
            <div
              class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {$useTurnServer ? 'translate-x-4' : 'translate-x-0.5'}"
            ></div>
          </button>
        </div>

        {#if error}
          <p class="text-xs text-red-400" transition:fade>{error}</p>
        {/if}
      </div>

    {:else}
      <!-- Connected -->
      <div class="space-y-3" in:fade>
        <!-- Room Code -->
        <div class="flex items-center gap-2 p-2 bg-white/5 rounded-lg">
          <div class="flex-1 min-w-0">
            <p class="text-[10px] text-white/50">Room Code</p>
            <p class="text-base font-bold font-mono tracking-widest text-[#1db954]">{$roomCode}</p>
          </div>
          <button
            class="p-1.5 rounded bg-white/10 hover:bg-white/20 transition-colors text-white/60 hover:text-white"
            onclick={copyRoomCode}
            title="Copy"
          >
            <Icon icon={copied ? "mdi:check" : "mdi:content-copy"} class="w-3.5 h-3.5" />
          </button>
        </div>

        <!-- Play Mode Selector -->
        <div>
          <p class="text-xs text-white/50 mb-1">Play Mode</p>
          <div class="flex gap-1">
            <button
              class="flex-1 px-3 py-1.5 rounded-lg text-xs transition-all flex items-center justify-center gap-1.5 {$bandPlayMode === 'split'
                ? 'bg-[#1db954] text-white font-medium'
                : 'bg-white/5 text-white/50 hover:bg-white/10'} {$isPlaying ? 'opacity-50 cursor-not-allowed' : ''}"
              onclick={() => $isHost && !$isPlaying && setBandPlayMode('split')}
              disabled={!$isHost || $isPlaying}
            >
              <Icon icon="mdi:call-split" class="w-3.5 h-3.5" />
              Split Notes
            </button>
            <button
              class="flex-1 px-3 py-1.5 rounded-lg text-xs transition-all flex items-center justify-center gap-1.5 {$bandPlayMode === 'track'
                ? 'bg-[#1db954] text-white font-medium'
                : 'bg-white/5 text-white/50 hover:bg-white/10'} {$isPlaying ? 'opacity-50 cursor-not-allowed' : ''}"
              onclick={() => $isHost && !$isPlaying && setBandPlayMode('track')}
              disabled={!$isHost || $isPlaying}
            >
              <Icon icon="mdi:music-note-eighth" class="w-3.5 h-3.5" />
              By Track
            </button>
          </div>
          <p class="text-[10px] text-white/40 mt-1">
            {#if $bandPlayMode === 'split'}
              Notes auto-distributed among players
            {:else}
              Each player picks a MIDI track
            {/if}
          </p>
        </div>

        <!-- Host Delay (Host only) -->
        {#if $isHost}
          <div>
            <div class="flex items-center justify-between mb-2">
              <p class="text-xs text-white/50">Sync Delay</p>
              <button
                class="px-2 py-0.5 rounded text-[10px] transition-all flex items-center gap-1 {$isCalibrating
                  ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30'
                  : 'bg-[#1db954]/20 text-[#1db954] hover:bg-[#1db954]/30'}"
                onclick={() => $isCalibrating ? stopCalibration() : startCalibration()}
                disabled={$connectedPeers.length < 2}
                title={$connectedPeers.length < 2 ? "Need at least 1 member to calibrate" : $isCalibrating ? "Stop calibration test" : "Play test beeps to calibrate sync timing"}
              >
                <Icon icon={$isCalibrating ? "mdi:stop" : "mdi:metronome"} class="w-3 h-3" />
                {$isCalibrating ? "Stop Test" : "Sync Test"}
              </button>
            </div>
            <!-- Input + Slider Row -->
            <div class="flex items-center gap-3">
              <div class="relative">
                <input
                  type="number"
                  min="-5000"
                  max="5000"
                  step="5"
                  bind:value={$hostDelay}
                  class="w-20 px-2 py-1 pr-7 bg-white/5 border border-white/10 rounded-lg text-xs text-white font-mono text-right focus:outline-none focus:ring-1 focus:ring-[#1db954] focus:border-transparent [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                />
                <span class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-white/40">ms</span>
              </div>
              <div class="flex-1">
                <div class="relative h-4 flex items-center">
                  <!-- Slider track background with gradient -->
                  <div class="absolute left-0 right-0 h-1.5 rounded-full overflow-hidden">
                    <div class="absolute inset-0 bg-gradient-to-r from-blue-500/30 via-white/10 to-orange-500/30"></div>
                    <!-- Active fill -->
                    <div
                      class="absolute h-full bg-[#1db954] transition-all rounded-full"
                      style="width: {(($hostDelay + 5000) / 10000) * 100}%"
                    ></div>
                  </div>
                  <!-- Tick marks -->
                  <div class="absolute left-0 right-0 flex justify-between items-center pointer-events-none">
                    <div class="w-px h-2.5 bg-white/20"></div>
                    <div class="w-px h-2.5 bg-white/30"></div>
                    <div class="w-px h-2.5 bg-white/20"></div>
                  </div>
                  <input
                    type="range"
                    min="-5000"
                    max="5000"
                    step="5"
                    bind:value={$hostDelay}
                    class="absolute inset-0 w-full appearance-none cursor-pointer bg-transparent
                      [&::-webkit-slider-thumb]:appearance-none
                      [&::-webkit-slider-thumb]:w-3.5
                      [&::-webkit-slider-thumb]:h-3.5
                      [&::-webkit-slider-thumb]:rounded-full
                      [&::-webkit-slider-thumb]:bg-[#1db954]
                      [&::-webkit-slider-thumb]:shadow-lg
                      [&::-webkit-slider-thumb]:shadow-[#1db954]/30
                      [&::-webkit-slider-thumb]:border-2
                      [&::-webkit-slider-thumb]:border-white
                      [&::-webkit-slider-thumb]:cursor-grab
                      [&::-webkit-slider-thumb]:active:cursor-grabbing
                      [&::-webkit-slider-thumb]:hover:scale-110
                      [&::-webkit-slider-thumb]:transition-transform
                      [&::-moz-range-thumb]:w-3.5
                      [&::-moz-range-thumb]:h-3.5
                      [&::-moz-range-thumb]:rounded-full
                      [&::-moz-range-thumb]:bg-[#1db954]
                      [&::-moz-range-thumb]:border-2
                      [&::-moz-range-thumb]:border-white
                      [&::-moz-range-thumb]:cursor-grab
                      [&::-moz-range-track]:bg-transparent
                      [&::-webkit-slider-runnable-track]:bg-transparent"
                  />
                </div>
                <!-- Labels directly under slider -->
                <div class="flex justify-between text-[9px] text-white/30 mt-0.5">
                  <span>-5s</span>
                  <span>0</span>
                  <span>+5s</span>
                </div>
              </div>
            </div>
            {#if $isCalibrating}
              <p class="text-[10px] text-yellow-400 mt-1.5 flex items-center gap-1">
                <Icon icon="mdi:pulse" class="w-3 h-3 animate-pulse" />
                Adjust until beeps sync across all devices
              </p>
            {:else}
              <p class="text-[10px] text-white/40 mt-1.5">
                + delay = host plays later · − delay = host plays earlier
              </p>
            {/if}
          </div>
        {/if}

        <!-- Song Selection (Host only) -->
        {#if $isHost}
          <div>
            <p class="text-xs text-white/50 mb-1">Song</p>
            {#if $bandSelectedSong}
              <button
                class="w-full flex items-center gap-2 p-1.5 rounded-lg bg-white/5 hover:bg-white/10 transition-colors text-left {$isPlaying ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}"
                onclick={() => !$isPlaying && handleSelectSong()}
                disabled={$isPlaying}
                title={$isPlaying ? "Stop playback to change song" : "Click to change song"}
              >
                <div class="w-7 h-7 rounded bg-white/10 flex items-center justify-center shrink-0">
                  <Icon icon="mdi:music-note" class="w-3.5 h-3.5 text-[#1db954]" />
                </div>
                <div class="flex-1 min-w-0">
                  <p class="text-xs font-medium truncate">{$bandSelectedSong.name}</p>
                </div>
                <Icon icon="mdi:chevron-right" class="w-4 h-4 text-white/30" />
              </button>
            {:else}
              <button
                class="w-full p-2 rounded-lg bg-white/5 hover:bg-white/10 border border-dashed border-white/20 text-white/50 hover:text-white transition-colors flex items-center justify-center gap-1.5 text-xs {$isPlaying ? 'opacity-50 cursor-not-allowed' : ''}"
                onclick={() => !$isPlaying && handleSelectSong()}
                disabled={$isPlaying}
              >
                <Icon icon="mdi:music-note-plus" class="w-3.5 h-3.5" />
                Select Song
              </button>
            {/if}
          </div>
        {:else}
          <!-- Song Display (Member view) -->
          <div>
            <p class="text-xs text-white/50 mb-1">Song</p>
            {#if $bandSelectedSong}
              <div class="flex items-center gap-2 p-1.5 rounded-lg bg-white/5">
                <div class="w-7 h-7 rounded bg-white/10 flex items-center justify-center shrink-0">
                  <Icon icon="mdi:music-note" class="w-3.5 h-3.5 text-[#1db954]" />
                </div>
                <div class="flex-1 min-w-0">
                  <p class="text-xs font-medium truncate">{$bandSelectedSong.name}</p>
                  {#if $bandSelectedSong.pending || !$bandFilePath}
                    <p class="text-[10px] text-yellow-400">Downloading...</p>
                  {:else}
                    <p class="text-[10px] text-white/40">Ready to play</p>
                  {/if}
                </div>
              </div>
            {:else}
              <div class="p-2 rounded-lg bg-white/5 border border-dashed border-white/20 text-white/40 text-center text-xs">
                Waiting for host to select a song...
              </div>
            {/if}
          </div>
        {/if}

        <!-- Players -->
        <div>
          <div class="flex items-center justify-between mb-1">
            <p class="text-xs text-white/50">
              {#if $bandPlayMode === 'split'}
                Players
              {:else}
                Players & Tracks
              {/if}
            </p>
            {#if $isHost && $bandPlayMode === 'track' && $availableTracks.length > 0 && !$isPlaying}
              <div class="flex gap-1">
                <button
                  class="px-2 py-0.5 rounded text-[10px] bg-[#1db954]/20 text-[#1db954] hover:bg-[#1db954]/30 transition-colors flex items-center gap-1"
                  onclick={autoAssignTracks}
                  title="Auto-assign tracks to players"
                >
                  <Icon icon="mdi:auto-fix" class="w-3 h-3" />
                  Auto
                </button>
                <button
                  class="px-2 py-0.5 rounded text-[10px] bg-white/10 text-white/50 hover:bg-white/20 hover:text-white transition-colors"
                  onclick={clearAllAssignments}
                  title="Clear all assignments"
                >
                  Clear
                </button>
              </div>
            {/if}
          </div>
          <div class="space-y-2">
            {#each $connectedPeers as peer, index (peer.id)}
              {@const peerTrack = $availableTracks.find(t => t.id === peer.trackId)}
              {@const slotNumber = peer.slot ?? index}
              <div
                class="p-2 rounded-lg bg-white/5"
                transition:fly={{ y: -5, duration: 150 }}
              >
                <!-- Player Info -->
                <div class="flex items-center gap-2">
                  {#if $bandPlayMode === 'split'}
                    <!-- Slot number badge -->
                    <div class="w-5 h-5 rounded-full bg-[#1db954] flex items-center justify-center text-[10px] font-bold text-black shrink-0">
                      {slotNumber + 1}
                    </div>
                  {:else}
                    <div class="w-5 h-5 rounded-full bg-white/10 flex items-center justify-center text-[9px] font-medium shrink-0">
                      {peer.name.charAt(0).toUpperCase()}
                    </div>
                  {/if}
                  <p class="text-xs font-medium truncate flex-1 flex items-center gap-1">
                    {peer.name}
                    {#if peer.isHost}
                      <Icon icon="mdi:crown" class="w-3 h-3 text-yellow-400" />
                    {/if}
                  </p>
                  <!-- Ready status indicator -->
                  {#if !peer.isHost && $bandSelectedSong}
                    <div class="flex items-center gap-1" title={peer.ready ? 'Ready' : 'Not ready'}>
                      <Icon
                        icon={peer.ready ? "mdi:check-circle" : "mdi:clock-outline"}
                        class="w-3.5 h-3.5 {peer.ready ? 'text-[#1db954]' : 'text-white/30'}"
                      />
                    </div>
                  {/if}
                  <span class="text-[9px] {getLatencyColor(peer.latency)}">{peer.latency}ms</span>
                  <!-- Kick button (host only, for non-host players) -->
                  {#if $isHost && !peer.isHost && !$isPlaying}
                    <button
                      class="p-1 rounded hover:bg-red-500/20 text-white/30 hover:text-red-400 transition-colors"
                      onclick={() => kickPlayer(peer.id)}
                      title="Kick player"
                    >
                      <Icon icon="mdi:close" class="w-3.5 h-3.5" />
                    </button>
                  {/if}
                </div>

                {#if $bandPlayMode === 'split'}
                  <!-- Split mode: show which notes this player gets -->
                  <p class="text-[10px] text-white/40 mt-1 ml-7">
                    Plays note {slotNumber + 1}, {slotNumber + 1 + $connectedPeers.length}, {slotNumber + 1 + $connectedPeers.length * 2}...
                  </p>
                {:else}
                  <!-- Track mode: track selection chips -->
                  {#if $availableTracks.length > 0}
                    <div class="flex flex-wrap gap-1 mt-2">
                      {#if $isHost && !$isPlaying}
                        <button
                          class="px-2 py-1 rounded text-[10px] transition-all {peer.trackId === null || peer.trackId === undefined
                            ? 'bg-white/20 text-white'
                            : 'bg-white/5 text-white/50 hover:bg-white/10'}"
                          onclick={() => assignTrackToPlayer(peer.id, '')}
                        >
                          All
                        </button>
                        {#each $availableTracks as track}
                          <button
                            class="px-2 py-1 rounded text-[10px] transition-all {peer.trackId === track.id
                              ? 'bg-[#1db954] text-white font-medium'
                              : 'bg-white/5 text-white/50 hover:bg-white/10'}"
                            onclick={() => assignTrackToPlayer(peer.id, track.id.toString())}
                          >
                            {track.name.length > 12 ? track.name.slice(0, 12) + '...' : track.name}
                          </button>
                        {/each}
                      {:else if peerTrack}
                        <span class="px-2 py-1 rounded text-[10px] bg-[#1db954] text-white font-medium">{peerTrack.name}</span>
                      {:else if $isPlaying && $isHost}
                        <span class="px-2 py-1 rounded text-[10px] bg-white/10 text-white/50">{peer.trackId !== null && peer.trackId !== undefined ? $availableTracks.find(t => t.id === peer.trackId)?.name || 'Track ' + peer.trackId : 'All'}</span>
                      {:else}
                        <span class="px-2 py-1 rounded text-[10px] bg-white/10 text-white/50">All</span>
                      {/if}
                    </div>
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        </div>

        <!-- Controls (Host only) -->
        {#if $isHost}
          {@const membersReady = $connectedPeers.filter(p => !p.isHost).every(p => p.ready)}
          {@const canPlay = $bandSelectedSong && $connectedPeers.length > 0 && membersReady}
          {@const showPause = $isPlaying && !$isPaused}
          {@const showResume = $isPlaying && $isPaused}
          <div class="flex gap-1.5">
            {#if showPause}
              <!-- Pause button when playing -->
              <button
                class="flex-1 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-xs transition-all flex items-center justify-center gap-1.5 active:scale-95"
                onclick={bandPause}
              >
                <Icon icon="mdi:pause" class="w-4 h-4" />
                Pause
              </button>
            {:else if showResume}
              <!-- Resume button when paused -->
              <button
                class="flex-1 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-xs transition-all flex items-center justify-center gap-1.5 active:scale-95"
                onclick={() => bandPlay(0)}
              >
                <Icon icon="mdi:play" class="w-4 h-4" />
                Resume
              </button>
            {:else}
              <!-- Play button when stopped -->
              <button
                class="flex-1 py-2 rounded-lg font-medium text-xs transition-all flex items-center justify-center gap-1.5 {canPlay
                  ? 'bg-[#1db954] hover:bg-[#1ed760] text-white active:scale-95 hover:shadow-lg hover:shadow-[#1db954]/20'
                  : 'bg-white/10 text-white/30 cursor-not-allowed'}"
                onclick={async () => {
                  if (!canPlay || isStartingPlayback) return;
                  isStartingPlayback = true;
                  try {
                    await bandPlay(0);
                  } finally {
                    setTimeout(() => isStartingPlayback = false, 500);
                  }
                }}
                disabled={!canPlay || isStartingPlayback}
                title={!$bandSelectedSong ? "Select a song first" : $connectedPeers.length === 0 ? "Waiting for players" : "Start synchronized playback"}
              >
                {#if isStartingPlayback}
                  <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
                  Starting...
                {:else}
                  <Icon icon="mdi:play" class="w-4 h-4" />
                  Play
                {/if}
              </button>
            {/if}
            <button
              class="py-2 px-3 rounded-lg transition-all flex items-center justify-center {$isPlaying
                ? 'bg-red-500/20 hover:bg-red-500/30 text-red-400'
                : 'bg-white/10 hover:bg-white/20 text-white/60 hover:text-white'} active:scale-95"
              onclick={bandStop}
              title="Stop"
              disabled={!$isPlaying}
            >
              <Icon icon="mdi:stop" class="w-4 h-4" />
            </button>
          </div>
          {#if !canPlay && !$isPlaying}
            <p class="text-[10px] text-white/40 mt-1 text-center">
              {#if !$bandSelectedSong}
                Select a song to play
              {:else if $connectedPeers.length === 0}
                Waiting for players to join...
              {:else if !membersReady}
                Waiting for all members to be ready...
              {/if}
            </p>
          {/if}
        {/if}

        <!-- Ready Button (Member only) -->
        {#if !$isHost && $bandSelectedSong}
          {@const isPending = $bandSelectedSong.pending || !$bandFilePath}
          <div class="space-y-2">
            {#if isPending}
              <div class="flex items-center justify-center gap-2 py-2 px-3 rounded-lg bg-yellow-500/10 text-yellow-400 text-xs">
                <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
                Receiving song file...
              </div>
            {:else}
              <button
                class="w-full py-2 rounded-lg font-medium text-xs transition-all flex items-center justify-center gap-1.5 {$myReady
                  ? 'bg-[#1db954] text-white'
                  : 'bg-white/10 text-white hover:bg-white/20'} {$isPlaying ? 'opacity-50 cursor-not-allowed' : 'active:scale-95'}"
                onclick={() => !$isPlaying && toggleReady()}
                disabled={$isPlaying}
              >
                <Icon icon={$myReady ? "mdi:check" : "mdi:hand-wave"} class="w-4 h-4" />
                {$myReady ? "Ready!" : "Ready Up"}
              </button>
              {#if !$myReady && !$isPlaying}
                <p class="text-[10px] text-white/40 text-center">
                  Click when you're ready to play
                </p>
              {/if}
            {/if}
          </div>
        {/if}

        <!-- Auto-Ready Toggle (Member only) -->
        {#if !$isHost}
          <div class="flex items-center justify-between p-2 rounded-lg bg-white/5">
            <div class="flex items-center gap-2">
              <Icon icon="mdi:lightning-bolt" class="w-3.5 h-3.5 text-white/50" />
              <span class="text-xs text-white/70">Auto-ready</span>
            </div>
            <button
              class="w-8 h-4 rounded-full transition-colors {$autoReady ? 'bg-[#1db954]' : 'bg-white/20'}"
              onclick={() => autoReady.update(v => !v)}
            >
              <div class="w-3 h-3 rounded-full bg-white transition-transform {$autoReady ? 'translate-x-4' : 'translate-x-0.5'}"></div>
            </button>
          </div>
        {/if}

        <!-- Leave -->
        <button
          class="w-full py-1.5 rounded-lg bg-white/5 hover:bg-white/10 text-white/50 hover:text-white text-xs transition-colors flex items-center justify-center gap-1.5"
          onclick={handleLeave}
        >
          <Icon icon="mdi:exit-run" class="w-3.5 h-3.5" />
          Leave
        </button>
      </div>
    {/if}
  </div>
</div>
