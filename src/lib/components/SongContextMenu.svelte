<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { invoke } from "../tauri/core-proxy.js";
  import { t } from "svelte-i18n";
  import { loadMidiFiles, removeDeletedFile } from "../stores/player.js";

  // Props
  export let contextMenu = null; // { x, y, file }
  export let onClose = () => {};

  // Internal state
  let showRenameModal = false;
  let renameValue = "";
  let renamingFile = null;
  let showDeleteModal = false;
  let deletingFile = null;

  function openRenameModal() {
    if (contextMenu?.file) {
      renamingFile = contextMenu.file;
      const name = renamingFile.name;
      renameValue = name.endsWith('.mid') ? name.slice(0, -4) : name;
      showRenameModal = true;
      onClose();
    }
  }

  async function handleRename() {
    if (!renamingFile || !renameValue.trim()) return;
    try {
      await invoke('rename_midi_file', {
        oldPath: renamingFile.path,
        newName: renameValue.trim()
      });
      await loadMidiFiles();
      showRenameModal = false;
      renamingFile = null;
      renameValue = "";
    } catch (err) {
      console.error('Failed to rename:', err);
    }
  }

  function openDeleteModal() {
    if (contextMenu?.file) {
      deletingFile = contextMenu.file;
      showDeleteModal = true;
      onClose();
    }
  }

  async function confirmDelete() {
    if (!deletingFile) return;
    try {
      // Remove from favorites and playlists first (before file is gone)
      if (deletingFile.hash) {
        removeDeletedFile(deletingFile.hash);
      }
      await invoke('delete_midi_file', { path: deletingFile.path });
      await loadMidiFiles();
    } catch (err) {
      console.error('Failed to delete:', err);
    }
    showDeleteModal = false;
    deletingFile = null;
  }

  async function handleOpenFolder() {
    if (!contextMenu?.file) return;
    try {
      await invoke('open_file_location', { path: contextMenu.file.path });
    } catch (err) {
      console.error('Failed to open folder:', err);
    }
    onClose();
  }
</script>

<!-- Context Menu -->
{#if contextMenu}
  <div
    class="fixed z-50 bg-[#282828] rounded-lg shadow-xl border border-white/10 py-1 min-w-[160px]"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
    transition:fly={{ y: -5, duration: 150 }}
    onclick={(e) => e.stopPropagation()}
  >
    <button
      class="w-full px-3 py-2 text-left text-sm text-white/80 hover:bg-white/10 flex items-center gap-2"
      onclick={openRenameModal}
    >
      <Icon icon="mdi:pencil" class="w-4 h-4" />
      {$t("contextMenu.rename")}
    </button>
    <button
      class="w-full px-3 py-2 text-left text-sm text-white/80 hover:bg-white/10 flex items-center gap-2"
      onclick={handleOpenFolder}
    >
      <Icon icon="mdi:folder-open" class="w-4 h-4" />
      {$t("contextMenu.openLocation")}
    </button>
    <div class="border-t border-white/10 my-1"></div>
    <button
      class="w-full px-3 py-2 text-left text-sm text-red-400 hover:bg-red-500/10 flex items-center gap-2"
      onclick={openDeleteModal}
    >
      <Icon icon="mdi:delete" class="w-4 h-4" />
      {$t("contextMenu.delete")}
    </button>
  </div>
{/if}

<!-- Delete Confirmation Modal -->
{#if showDeleteModal && deletingFile}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { showDeleteModal = false; deletingFile = null; }}
    ></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[360px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <div class="p-4 text-center">
        <div class="w-12 h-12 rounded-full bg-red-500/20 flex items-center justify-center mx-auto mb-3">
          <Icon icon="mdi:delete-alert" class="w-6 h-6 text-red-400" />
        </div>
        <h3 class="text-lg font-bold mb-2">{$t("modals.deleteSong.title")}</h3>
        <p class="text-sm text-white/60 mb-1">"{deletingFile.name}"</p>
        <p class="text-xs text-white/40">{$t("modals.deleteSong.cannotBeUndone")}</p>
      </div>

      <div class="flex gap-2 p-4 pt-0">
        <button
          class="flex-1 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition-colors"
          onclick={() => { showDeleteModal = false; deletingFile = null; }}
        >
          {$t("common.cancel")}
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-red-500 hover:bg-red-600 text-white font-medium text-sm transition-colors"
          onclick={confirmDelete}
        >
          {$t("common.delete")}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Rename Modal -->
{#if showRenameModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    transition:fade={{ duration: 150 }}
  >
    <button
      class="absolute inset-0 bg-black/60"
      onclick={() => { showRenameModal = false; renamingFile = null; }}
    ></button>

    <div
      class="relative bg-[#282828] rounded-xl shadow-2xl w-[360px] max-w-[90vw] overflow-hidden"
      transition:fly={{ y: 20, duration: 200 }}
    >
      <div class="flex items-center justify-between p-4 border-b border-white/10">
        <h3 class="text-lg font-bold">{$t("modals.rename.title")}</h3>
        <button
          class="p-1 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors"
          onclick={() => { showRenameModal = false; renamingFile = null; }}
        >
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <div class="p-4">
        <input
          type="text"
          bind:value={renameValue}
          class="w-full px-4 py-2.5 bg-white/5 border border-white/10 rounded-lg text-sm text-white placeholder-white/30 focus:outline-none focus:ring-2 focus:ring-[#1db954] focus:border-transparent transition-all"
          onkeydown={(e) => e.key === 'Enter' && handleRename()}
          autofocus
        />
        <p class="text-xs text-white/40 mt-2">{$t("modals.rename.extensionNote")}</p>
      </div>

      <div class="flex gap-2 p-4 pt-0">
        <button
          class="flex-1 py-2 rounded-lg bg-white/10 hover:bg-white/20 text-white font-medium text-sm transition-colors"
          onclick={() => { showRenameModal = false; renamingFile = null; }}
        >
          {$t("common.cancel")}
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-[#1db954] hover:bg-[#1ed760] text-white font-medium text-sm transition-colors"
          onclick={handleRename}
        >
          {$t("contextMenu.rename")}
        </button>
      </div>
    </div>
  </div>
{/if}
