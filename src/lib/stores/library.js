import { writable, get } from 'svelte/store';
import { invoke } from '../tauri/core-proxy.js';
import Peer from 'peerjs';
import { logUiAction } from '../utils/uiActionLogger.js';

// Discovery server URL (configurable in developer mode)
export const discoveryServerUrl = writable('https://discovery.chuaii.me');
export const developerMode = writable(false);
export const isHostingServer = writable(false);

// Library state
export const libraryEnabled = writable(false); // Disabled by default until server is set up
export const libraryConnected = writable(false);
export const globalSongs = writable([]); // All songs from all peers
export const onlinePeers = writable(0);
export const downloadProgress = writable(null);
export const libraryError = writable(null);
export const shareAll = writable(false);
export const sharedSongs = writable([]);
export const shareNotification = writable(null); // { songName, peerName, timestamp }

// Internal state
let peer = null;
let myPeerId = null;
let myClientId = null; // Persistent ID for discovery server
let heartbeatInterval = null;
let fetchInterval = null;
let initialized = false;

const HEARTBEAT_INTERVAL = 15000; // 15 seconds - updates your shared songs to server
const FETCH_INTERVAL = 15000; // Refresh song list every 15s

// Get or create persistent client ID
function getClientId() {
  if (myClientId) return myClientId;

  let id = localStorage.getItem('libraryClientId');
  if (!id) {
    id = crypto.randomUUID();
    localStorage.setItem('libraryClientId', id);
  }
  myClientId = id;
  return id;
}

// Get discovery server URL
function getServerUrl() {
  return get(discoveryServerUrl);
}

// Get our shareable songs list
async function getShareableSongs() {
  const $shareAll = get(shareAll);
  const $sharedSongs = get(sharedSongs);

  try {
    const allSongs = await invoke('load_midi_files');

    if ($shareAll) {
      return allSongs.map(s => ({
        name: s.name,
        hash: s.hash || hashString(s.path),
        duration: s.duration,
        bpm: s.bpm,
        size: s.size || 0
      }));
    } else {
      return allSongs
        .filter(s => $sharedSongs.includes(s.path))
        .map(s => ({
          name: s.name,
          hash: s.hash || hashString(s.path),
          duration: s.duration,
          bpm: s.bpm,
          size: s.size || 0
        }));
    }
  } catch (err) {
    console.error('[LIBRARY] Failed to get songs:', err);
    return [];
  }
}

// Simple string hash
function hashString(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash;
  }
  return Math.abs(hash).toString(16);
}

// Generate display name
function getDisplayName() {
  const adjectives = ['Happy', 'Swift', 'Calm', 'Brave', 'Wise', 'Kind'];
  const nouns = ['Musician', 'Player', 'Artist', 'Bard', 'Minstrel', 'Maestro'];
  const adj = adjectives[Math.floor(Math.random() * adjectives.length)];
  const noun = nouns[Math.floor(Math.random() * nouns.length)];
  const num = Math.floor(Math.random() * 100);
  return `${adj}${noun}${num}`;
}

// Connect to library (discovery server + PeerJS)
export async function connectLibrary() {
  if (peer) {
    logUiAction('library.connect', 'warn', { reason: 'already_connected' });
    return;
  }

  const enabled = get(libraryEnabled);
  if (!enabled) {
    logUiAction('library.connect', 'warn', { reason: 'disabled' });
    return;
  }

  const actionContext = { server: getServerUrl(), shareAll: get(shareAll) };
  logUiAction('library.connect', 'started', actionContext);

  const myName = localStorage.getItem('libraryName') || getDisplayName();
  localStorage.setItem('libraryName', myName);

  try {
    // Create PeerJS connection for file transfers
    peer = new Peer({ debug: 0 });

    peer.on('open', async (id) => {
      console.log('[LIBRARY] PeerJS connected:', id);
      myPeerId = id;

      // Register with discovery server
      const registered = await registerWithServer();

      if (registered) {
        libraryConnected.set(true);
        libraryError.set(null);

        logUiAction('library.connect', 'completed', {
          ...actionContext,
          peerId: id,
          registered: true
        });

        // Start heartbeat and fetch intervals
        startHeartbeat();
        startFetchInterval();

        // Initial fetch
        await fetchGlobalSongs();
      } else {
        libraryError.set('Cannot connect to discovery server');
        logUiAction('library.connect', 'error', {
          ...actionContext,
          peerId: id,
          reason: 'registration_failed'
        });
      }
    });

    peer.on('connection', (conn) => handleIncomingConnection(conn));

    peer.on('error', (err) => {
      console.error('[LIBRARY] PeerJS error:', err);
      if (err.type !== 'peer-unavailable') {
        libraryError.set(err.message);
      }
    });

    peer.on('disconnected', () => {
      console.log('[LIBRARY] PeerJS disconnected, reconnecting...');
      if (peer && !peer.destroyed) {
        peer.reconnect();
      }
    });

    peer.on('close', () => {
      console.log('[LIBRARY] PeerJS closed');
      peer = null;
      myPeerId = null;
      libraryConnected.set(false);
    });

  } catch (err) {
    console.error('[LIBRARY] Failed to connect:', err);
    logUiAction('library.connect', 'error', {
      ...actionContext,
      error: err?.message || err
    });
    libraryError.set(err.toString());
  }
}

// Register with discovery server - returns true on success
async function registerWithServer() {
  if (!myPeerId) return false;

  const clientId = getClientId();
  const myName = localStorage.getItem('libraryName') || 'Unknown';
  const songs = await getShareableSongs();

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 5000);

    const response = await fetch(`${getServerUrl()}/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        peer_id: clientId,
        webrtc_id: myPeerId,
        name: myName,
        songs
      }),
      signal: controller.signal
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}`);
    }

    console.log('[LIBRARY] Registered with discovery server');
    return true;
  } catch (err) {
    console.error('[LIBRARY] Failed to register:', err);
    return false;
  }
}

// Fetch global song list from discovery server
async function fetchGlobalSongs() {
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 5000);

    const response = await fetch(`${getServerUrl()}/peers`, { signal: controller.signal });
    clearTimeout(timeoutId);

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}`);
    }

    const data = await response.json();

    // Flatten all songs from all peers (excluding our own)
    const allSongs = [];
    let peerCount = 0;
    const clientId = getClientId();

    for (const peer of data.peers) {
      if (peer.peer_id === clientId) continue; // Skip our own songs
      if (!peer.webrtc_id) continue; // Skip peers without WebRTC ID
      peerCount++;

      for (const song of peer.songs) {
        allSongs.push({
          ...song,
          peerId: peer.webrtc_id, // Use WebRTC ID for P2P connection
          peerName: peer.name
        });
      }
    }

    globalSongs.set(allSongs);
    onlinePeers.set(peerCount);

    console.log(`[LIBRARY] Fetched ${allSongs.length} songs from ${peerCount} peers`);
    if (allSongs.length > 0) {
      console.log('[LIBRARY] Sample song:', allSongs[0]);
    }
  } catch (err) {
    console.error('[LIBRARY] Failed to fetch songs:', err);
    // Don't clear existing songs on error
  }
}

// Start periodic heartbeat to discovery server
function startHeartbeat() {
  if (heartbeatInterval) clearInterval(heartbeatInterval);

  heartbeatInterval = setInterval(async () => {
    if (!myPeerId) return;

    const clientId = getClientId();
    const myName = localStorage.getItem('libraryName') || 'Unknown';
    const songs = await getShareableSongs();

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      await fetch(`${getServerUrl()}/heartbeat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          peer_id: clientId,
          webrtc_id: myPeerId,
          name: myName,
          songs
        }),
        signal: controller.signal
      });

      clearTimeout(timeoutId);
    } catch (err) {
      console.error('[LIBRARY] Heartbeat failed:', err);
    }
  }, HEARTBEAT_INTERVAL);
}

// Start periodic fetch of global song list
function startFetchInterval() {
  if (fetchInterval) clearInterval(fetchInterval);

  fetchInterval = setInterval(() => {
    fetchGlobalSongs();
  }, FETCH_INTERVAL);
}

// Handle incoming P2P connection (for file requests)
function handleIncomingConnection(conn) {
  console.log('[LIBRARY] Incoming connection from:', conn.peer);

  let peerName = 'Someone'; // Will be updated if peer sends their name

  conn.on('open', () => {
    conn.on('data', async (data) => {
      if (data.type === 'request_song') {
        // Store peer name if provided
        if (data.peerName) {
          peerName = data.peerName;
        }
        await handleSongRequest(data.hash, conn, peerName);
      }
    });
  });

  conn.on('error', (err) => {
    console.error('[LIBRARY] Connection error:', err);
  });
}

// Handle song request from peer
async function handleSongRequest(hash, conn, peerName = 'Someone') {
  try {
    const allSongs = await invoke('load_midi_files');
    const song = allSongs.find(s => (s.hash || hashString(s.path)) === hash);

    if (!song) {
      conn.send({ type: 'song_error', hash, error: 'Song not found' });
      return;
    }

    // Check if we're sharing this song
    const $shareAll = get(shareAll);
    const $sharedSongs = get(sharedSongs);
    if (!$shareAll && !$sharedSongs.includes(song.path)) {
      conn.send({ type: 'song_error', hash, error: 'Song not shared' });
      return;
    }

    // Read and send the file
    const fileData = await invoke('read_midi_base64', { path: song.path });
    conn.send({
      type: 'song_data',
      hash,
      name: song.name,
      filename: song.path.split(/[\\/]/).pop(),
      data: fileData
    });

    console.log('[LIBRARY] Sent song:', song.name, 'to', peerName);

    // Emit notification that someone downloaded our song
    shareNotification.set({
      songName: song.name,
      peerName: peerName,
      timestamp: Date.now()
    });
  } catch (err) {
    console.error('[LIBRARY] Failed to send song:', err);
    conn.send({ type: 'song_error', hash, error: err.toString() });
  }
}

// Request a song from a peer (P2P download)
export async function requestSong(peerId, hash, songName) {
  const actionContext = {
    peerId,
    hash,
    songName
  };
  if (!peer) {
    libraryError.set('Not connected');
    logUiAction('library.requestSong', 'error', {
      ...actionContext,
      reason: 'not_connected'
    });
    return false;
  }

  downloadProgress.set({ songName, progress: 10, status: 'Connecting...' });
  logUiAction('library.requestSong', 'started', actionContext);

  try {
    const conn = peer.connect(peerId);

    return new Promise((resolve) => {
      const timeout = setTimeout(() => {
        downloadProgress.set(null);
        libraryError.set('Connection timeout');
        logUiAction('library.requestSong', 'error', {
          ...actionContext,
          reason: 'timeout'
        });
        resolve(false);
      }, 15000);

      conn.on('open', () => {
        downloadProgress.set({ songName, progress: 20, status: 'Requesting...' });
        const myName = localStorage.getItem('libraryName') || 'Someone';
        conn.send({ type: 'request_song', hash, peerName: myName });
      });

      conn.on('data', async (data) => {
        clearTimeout(timeout);

        if (data.type === 'song_data') {
          logUiAction('library.requestSong', 'completed', {
            ...actionContext,
            response: 'song_data'
          });
          await handleSongData(data);
          conn.close();
          resolve(true);
        } else if (data.type === 'song_error') {
          downloadProgress.set(null);
          libraryError.set(data.error);
          logUiAction('library.requestSong', 'error', {
            ...actionContext,
            error: data.error
          });
          conn.close();
          resolve(false);
        }
      });

      conn.on('error', (err) => {
        clearTimeout(timeout);
        downloadProgress.set(null);
        libraryError.set(err.toString());
        resolve(false);
      });
    });
  } catch (err) {
    downloadProgress.set(null);
    libraryError.set(err.toString());
    return false;
  }
}

// Handle received song data
async function handleSongData(data) {
  try {
    downloadProgress.set({ songName: data.name, progress: 50, status: 'Verifying...' });

    const isValid = await invoke('verify_midi_data', { dataBase64: data.data });
    if (!isValid) {
      throw new Error('Invalid MIDI file');
    }

    downloadProgress.set({ songName: data.name, progress: 80, status: 'Saving...' });

    await invoke('save_midi_from_base64', {
      filename: data.filename,
      dataBase64: data.data
    });

    downloadProgress.set({ songName: data.name, progress: 100, status: 'Complete!' });

    // Refresh file list
    const { loadMidiFiles } = await import('./player.js');
    await loadMidiFiles();

    setTimeout(() => downloadProgress.set(null), 2000);
  } catch (err) {
    console.error('[LIBRARY] Failed to save song:', err);
    libraryError.set(err.toString());
    downloadProgress.set(null);
  }
}

// Disconnect from library
export async function disconnectLibrary() {
  // Unregister from discovery server
  const clientId = getClientId();
  if (clientId) {
    try {
      await fetch(`${getServerUrl()}/unregister`, {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(clientId)
      });
    } catch (err) {
      console.error('[LIBRARY] Failed to unregister:', err);
    }
  }

  if (heartbeatInterval) {
    clearInterval(heartbeatInterval);
    heartbeatInterval = null;
  }

  if (fetchInterval) {
    clearInterval(fetchInterval);
    fetchInterval = null;
  }

  if (peer) {
    peer.destroy();
    peer = null;
  }

  myPeerId = null;
  libraryConnected.set(false);
  globalSongs.set([]);
  onlinePeers.set(0);
}

// Toggle library
export function toggleLibrary() {
  const enabled = !get(libraryEnabled);
  libraryEnabled.set(enabled);
  localStorage.setItem('libraryEnabled', enabled.toString());

  if (enabled) {
    libraryError.set(null);
    connectLibrary();
  } else {
    disconnectLibrary();
  }
}

// Set shared songs
export function setSharedSongs(paths) {
  sharedSongs.set(paths);
  localStorage.setItem('sharedSongs', JSON.stringify(paths));
}

// Toggle share all
export function toggleShareAll() {
  const newValue = !get(shareAll);
  shareAll.set(newValue);
  localStorage.setItem('shareAll', newValue.toString());
}

// Set discovery server URL
export function setDiscoveryServer(url) {
  discoveryServerUrl.set(url);
  localStorage.setItem('discoveryServerUrl', url);

  // Reconnect if connected
  if (get(libraryConnected)) {
    disconnectLibrary().then(() => connectLibrary());
  }
}

// Toggle developer mode
export function toggleDeveloperMode() {
  const newValue = !get(developerMode);
  developerMode.set(newValue);
  localStorage.setItem('developerMode', newValue.toString());
}

// Start discovery server (developer mode)
export async function startServer(port = 3456) {
  try {
    await invoke('start_discovery_server', { port });
    isHostingServer.set(true);
    console.log('[LIBRARY] Discovery server started on port', port);
    return true;
  } catch (err) {
    console.error('[LIBRARY] Failed to start server:', err);
    libraryError.set(err.toString());
    return false;
  }
}

// Stop discovery server
export async function stopServer() {
  try {
    await invoke('stop_discovery_server');
    isHostingServer.set(false);
    console.log('[LIBRARY] Discovery server stopped');
    return true;
  } catch (err) {
    console.error('[LIBRARY] Failed to stop server:', err);
    libraryError.set(err.toString());
    return false;
  }
}

// Initialize library
export function initLibrary() {
  if (initialized) return;
  initialized = true;

  // Load settings from localStorage
  const savedEnabled = localStorage.getItem('libraryEnabled');
  if (savedEnabled !== null) {
    libraryEnabled.set(savedEnabled === 'true');
  }

  const savedShareAll = localStorage.getItem('shareAll');
  if (savedShareAll !== null) {
    shareAll.set(savedShareAll === 'true');
  }

  const savedShared = localStorage.getItem('sharedSongs');
  if (savedShared) {
    try {
      sharedSongs.set(JSON.parse(savedShared));
    } catch (e) {}
  }

  const savedServerUrl = localStorage.getItem('discoveryServerUrl');
  if (savedServerUrl) {
    discoveryServerUrl.set(savedServerUrl);
  }

  const savedDevMode = localStorage.getItem('developerMode');
  if (savedDevMode !== null) {
    developerMode.set(savedDevMode === 'true');
  }

  // Auto-connect if enabled
  if (get(libraryEnabled)) {
    setTimeout(() => connectLibrary(), 1000);
  }
}

// Get my peer ID
export function getMyPeerId() {
  return myPeerId;
}

// Manually refresh song list
export async function refreshSongs() {
  await fetchGlobalSongs();
}
