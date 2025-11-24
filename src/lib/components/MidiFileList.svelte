<script>
  import Icon from "@iconify/svelte";
  import {
    midiFiles,
    currentFile,
    playMidi,
    playlist,
    isPlaying,
    isPaused,
  } from "../stores/player.js";

  let searchQuery = "";

  async function handlePlay(file) {
    await playMidi(file.path);
  }

  function addToPlaylist(file) {
    playlist.update((list) => {
      if (!list.find((f) => f.path === file.path)) {
        return [...list, file];
      }
      return list;
    });
  }

  $: filteredFiles = $midiFiles.filter((file) =>
    file.name.toLowerCase().includes(searchQuery.toLowerCase()),
  );
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="mb-4">
    <h2 class="text-2xl font-bold mb-2">Your Library</h2>
    <p class="text-sm text-white/60 mb-4">
      {filteredFiles.length} of {$midiFiles.length} songs
    </p>

    <!-- Search Input -->
    <div class="">
      <div class="relative">
        <Icon
          icon="mdi:magnify"
          class="absolute left-3 top-1/2 -translate-y-1/2 text-white/60 w-5 h-5"
        />
        <input
          type="text"
          placeholder="Search songs..."
          bind:value={searchQuery}
          class="w-full bg-white/10 border border-white/10 rounded-lg pl-10 pr-4 py-2 text-sm text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent"
        />
        {#if searchQuery}
          <button
            on:click={() => (searchQuery = "")}
            class="absolute right-3 top-1/2 -translate-y-1/2 text-white/60 hover:text-white"
          >
            <Icon icon="mdi:close" class="w-5 h-5" />
          </button>
        {/if}
      </div>
    </div>
  </div>

  <!-- Song List (Scrollable) -->
  <div class="flex-1 overflow-y-auto space-y-1 pr-2">
    {#each filteredFiles as file, index}
      <div
        class="group spotify-list-item flex items-center gap-4 py-2 {$currentFile ===
        file.path
          ? 'bg-white/10'
          : ''}"
      >
        <!-- Number / Play Button / Playing Indicator -->
        <div class="w-8 flex items-center justify-center flex-shrink-0">
          {#if $currentFile === file.path && $isPlaying && !$isPaused}
            <!-- Playing indicator (animated bars) -->
            <div class="flex items-end gap-0.5 h-4">
              <div
                class="w-0.5 bg-[#1db954] animate-music-bar-1"
                style="height: 60%; animation: music-bar-1 0.6s ease-in-out infinite;"
              ></div>
              <div
                class="w-0.5 bg-[#1db954] animate-music-bar-2"
                style="height: 100%; animation: music-bar-2 0.8s ease-in-out infinite;"
              ></div>
              <div
                class="w-0.5 bg-[#1db954] animate-music-bar-3"
                style="height: 80%; animation: music-bar-3 0.7s ease-in-out infinite;"
              ></div>
            </div>
          {:else}
            <span
              class="text-sm text-white/60 {$currentFile === file.path
                ? 'text-[#1db954]'
                : ''} group-hover:hidden">{index + 1}</span
            >
            <button
              class="hidden group-hover:block"
              on:click={() => handlePlay(file)}
              title="Play"
            >
              <svg
                class="w-4 h-4 text-white"
                fill="currentColor"
                viewBox="0 0 24 24"
              >
                <path d="M8 5v14l11-7z" />
              </svg>
            </button>
          {/if}
        </div>

        <!-- Song Info -->
        <div
          class="flex-1 min-w-0"
          role="button"
          tabindex="0"
          on:click={() => handlePlay(file)}
          on:keydown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              handlePlay(file);
            }
          }}
        >
          <p
            class="text-sm font-medium text-white truncate {$currentFile ===
            file.path
              ? 'text-[#1db954]'
              : ''}"
          >
            {file.name}
          </p>
          <p class="text-xs text-white/60">MIDI Track</p>
        </div>

        <!-- Duration -->
        <div class="text-sm text-white/60 flex-shrink-0">
          {file.duration
            ? `${Math.floor(file.duration / 60)}:${String(Math.floor(file.duration % 60)).padStart(2, "0")}`
            : "--:--"}
        </div>

        <!-- Add to Playlist -->
        <button
          class="opacity-0 group-hover:opacity-100 text-white/60 hover:text-white transition-all flex-shrink-0"
          on:click={() => addToPlaylist(file)}
          title="Add to playlist"
        >
          <svg
            class="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 4v16m8-8H4"
            />
          </svg>
        </button>
      </div>
    {/each}
  </div>

  {#if filteredFiles.length === 0 && searchQuery}
    <div class="text-center py-16 text-white/50">
      <Icon
        icon="mdi:music-note-off"
        class="w-16 h-16 mx-auto mb-4 opacity-50"
      />
      <p class="text-base font-semibold mb-2">No results found</p>
      <p class="text-sm">Try a different search term</p>
    </div>
  {:else if $midiFiles.length === 0}
    <div class="text-center py-16 text-white/50">
      <Icon
        icon="mdi:music-note-plus"
        class="w-16 h-16 mx-auto mb-4 opacity-50"
      />
      <p class="text-base font-semibold mb-2">No songs yet</p>
      <p class="text-sm">Place MIDI files in the album folder</p>
    </div>
  {/if}
</div>
