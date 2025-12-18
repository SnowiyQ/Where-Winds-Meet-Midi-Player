import { writable, get } from 'svelte/store';
import { invoke } from '../tauri/core-proxy.js';
import Peer from 'peerjs';

// Import player stores for syncing
import {
  speed,
  noteMode,
  keyMode,
  octaveShift,
  setNoteMode,
  setKeyMode,
  setOctaveShift,
  setSpeed
} from './player.js';

// Store original settings before joining band (to restore on leave)
let originalSettings = null;

function saveOriginalSettings() {
  originalSettings = {
    speed: get(speed),
    noteMode: get(noteMode),
    keyMode: get(keyMode),
    octaveShift: get(octaveShift)
  };
  console.log('[BAND] Saved original settings:', originalSettings);
}

async function restoreOriginalSettings() {
  if (originalSettings) {
    console.log('[BAND] Restoring original settings:', originalSettings);
    await setSpeed(originalSettings.speed);
    await setNoteMode(originalSettings.noteMode);
    await setKeyMode(originalSettings.keyMode);
    await setOctaveShift(originalSettings.octaveShift);
    originalSettings = null;
  }
}

// Band mode state
export const bandEnabled = writable(false);
export const isHost = writable(false);
export const roomCode = writable(null);
export const connectedPeers = writable([]); // [{ id, name, latency, trackId, slot, ready }]
export const myTrackId = writable(null);
export const mySlot = writable(null); // For split mode: which slot (0, 1, 2...) this player has
export const availableTracks = writable([]); // [{ id, name, noteCount }]
export const bandStatus = writable('disconnected'); // disconnected, connecting, connected, error
export const bandSongSelectMode = writable(false); // true when selecting song for band
export const bandSelectedSong = writable(null); // { name, path, ... }
export const bandPlayMode = writable('split'); // 'split' = auto-distribute notes, 'track' = each player picks a track
export const myReady = writable(false); // Member's ready state
export const bandFilePath = writable(null); // Path to use for playback (local or temp)
export const autoReady = writable(true); // Auto-ready when song is received
export const isCalibrating = writable(false); // Calibration mode active

// Load saved hostDelay from localStorage
const savedHostDelay = typeof localStorage !== 'undefined'
  ? parseInt(localStorage.getItem('hostDelay')) || 300
  : 300;
export const hostDelay = writable(savedHostDelay);

// Persist hostDelay changes to localStorage
hostDelay.subscribe(value => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('hostDelay', value.toString());
  }
});

// TURN server settings (for users who can't connect directly)
const savedUseTurn = typeof localStorage !== 'undefined'
  ? localStorage.getItem('useTurnServer') === 'true'
  : false;
export const useTurnServer = writable(savedUseTurn);

useTurnServer.subscribe(value => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('useTurnServer', value.toString());
  }
});

// Get ICE configuration based on TURN setting
function getIceConfig() {
  const config = {
    iceServers: [
      { urls: 'stun:stun.l.google.com:19302' },
      { urls: 'stun:stun1.l.google.com:19302' },
      { urls: 'stun:stun2.l.google.com:19302' },
    ]
  };

  if (get(useTurnServer)) {
    // Use OpenRelay free public TURN servers
    config.iceServers.push(
      {
        urls: 'turn:openrelay.metered.ca:80',
        username: 'openrelayproject',
        credential: 'openrelayproject'
      },
      {
        urls: 'turn:openrelay.metered.ca:443',
        username: 'openrelayproject',
        credential: 'openrelayproject'
      },
      {
        urls: 'turn:openrelay.metered.ca:443?transport=tcp',
        username: 'openrelayproject',
        credential: 'openrelayproject'
      }
    );
    console.log('[BAND] Using TURN relay servers');
  }

  return config;
}

// Internal state
let peer = null;
let connections = new Map(); // peerId -> DataConnection
let latencyIntervals = new Map();
let syncInterval = null;
let calibrationInterval = null;

// Generate short room code
function generateRoomCode() {
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'; // No confusing chars (0,O,1,I)
  let code = '';
  for (let i = 0; i < 6; i++) {
    code += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return code;
}

// Create a room (host)
export async function createRoom(playerName = 'Host') {
  return new Promise((resolve, reject) => {
    const code = generateRoomCode();

    // PeerJS uses the room code as the peer ID for easy joining
    const iceConfig = getIceConfig();
    console.log('[BAND] Creating room with ICE config:', iceConfig);
    peer = new Peer(`wwm-${code}`, {
      debug: 1,
      config: iceConfig,
    });

    peer.on('open', (id) => {
      console.log('Room created:', code);
      isHost.set(true);
      roomCode.set(code);
      bandStatus.set('connected');

      // Add self to peers
      connectedPeers.set([{
        id: 'host',
        name: playerName,
        latency: 0,
        trackId: null,
        isHost: true,
        ready: true // Host is always ready
      }]);

      resolve(code);
    });

    peer.on('connection', (conn) => {
      handleIncomingConnection(conn);
    });

    peer.on('error', (err) => {
      console.error('Peer error:', err);
      bandStatus.set('error');
      reject(err);
    });

    peer.on('disconnected', () => {
      bandStatus.set('disconnected');
    });
  });
}

// Join a room (player)
export async function joinRoom(code, playerName = 'Player') {
  return new Promise((resolve, reject) => {
    code = code.toUpperCase().trim();

    let connectionTimeout = null;
    let connected = false;

    const iceConfig = getIceConfig();
    console.log('[BAND] Joining room with ICE config:', iceConfig);
    peer = new Peer({
      debug: 1,
      config: iceConfig,
    });

    const cleanup = () => {
      if (connectionTimeout) {
        clearTimeout(connectionTimeout);
        connectionTimeout = null;
      }
    };

    peer.on('open', () => {
      bandStatus.set('connecting');

      // Set connection timeout (10 seconds)
      connectionTimeout = setTimeout(() => {
        if (!connected) {
          console.error('Connection timeout - room may not exist');
          bandStatus.set('error');
          if (peer) {
            peer.destroy();
            peer = null;
          }
          reject(new Error('Room not found or connection timeout'));
        }
      }, 10000);

      // Connect to the host
      const conn = peer.connect(`wwm-${code}`, {
        metadata: { name: playerName }
      });

      conn.on('open', () => {
        connected = true;
        cleanup();

        console.log('Connected to room:', code);

        // Save original settings before joining (to restore on leave)
        saveOriginalSettings();

        isHost.set(false);
        roomCode.set(code);
        bandStatus.set('connected');

        connections.set('host', conn);
        setupConnectionHandlers(conn, 'host');

        // Send join message
        conn.send({
          type: 'join',
          name: playerName
        });

        resolve(code);
      });

      conn.on('error', (err) => {
        cleanup();
        console.error('Connection error:', err);
        bandStatus.set('error');
        reject(err);
      });
    });

    peer.on('error', (err) => {
      cleanup();
      console.error('Peer error:', err);
      if (err.type === 'peer-unavailable') {
        bandStatus.set('error');
        reject(new Error('Room not found'));
      } else {
        bandStatus.set('error');
        reject(err);
      }
    });
  });
}

// Handle incoming connection (host only)
function handleIncomingConnection(conn) {
  const peerId = conn.peer;

  conn.on('open', () => {
    console.log('Player connected:', peerId);
    connections.set(peerId, conn);
    setupConnectionHandlers(conn, peerId);
  });
}

// Setup message handlers for a connection
function setupConnectionHandlers(conn, peerId) {
  conn.on('data', (data) => {
    handleMessage(data, peerId, conn);
  });

  conn.on('close', () => {
    console.log('Peer disconnected:', peerId);
    connections.delete(peerId);
    latencyIntervals.delete(peerId);

    connectedPeers.update(peers =>
      peers.filter(p => p.id !== peerId)
    );

    // Notify others if host
    if (get(isHost)) {
      broadcast({ type: 'peer_left', peerId });
    }
  });

  // Start latency measurement
  startLatencyMeasurement(peerId, conn);
}

// Handle incoming messages
function handleMessage(data, fromPeerId, conn) {
  const $isHost = get(isHost);

  switch (data.type) {
    case 'join':
      if ($isHost) {
        // Add new player
        const newPeer = {
          id: fromPeerId,
          name: data.name,
          latency: 0,
          trackId: null,
          isHost: false,
          ready: false
        };

        connectedPeers.update(peers => [...peers, newPeer]);

        // Send current state to new player
        const $bandSelectedSong = get(bandSelectedSong);
        conn.send({
          type: 'room_state',
          peers: get(connectedPeers),
          tracks: get(availableTracks),
          mode: get(bandPlayMode),
          song: $bandSelectedSong ? { name: $bandSelectedSong.name, filename: $bandSelectedSong.filename } : null
        });

        // If song is selected, send file data for the new player
        if ($bandSelectedSong && $bandSelectedSong.fileData) {
          conn.send({
            type: 'song_data',
            filename: $bandSelectedSong.filename,
            fileData: $bandSelectedSong.fileData
          });
        }

        // Notify others
        broadcast({ type: 'peer_joined', peer: newPeer }, fromPeerId);
      }
      break;

    case 'room_state':
      // Received room state from host
      connectedPeers.set(data.peers);
      availableTracks.set(data.tracks);
      if (data.mode) bandPlayMode.set(data.mode);
      break;

    case 'peer_joined':
      connectedPeers.update(peers => [...peers, data.peer]);
      break;

    case 'peer_left':
      connectedPeers.update(peers =>
        peers.filter(p => p.id !== data.peerId)
      );
      break;

    case 'ping':
      // Respond to latency ping
      conn.send({
        type: 'pong',
        timestamp: data.timestamp
      });
      break;

    case 'pong':
      // Calculate latency
      const latency = (Date.now() - data.timestamp) / 2;
      connectedPeers.update(peers =>
        peers.map(p => p.id === fromPeerId ? { ...p, latency: Math.round(latency) } : p)
      );
      break;

    case 'track_assign':
      // Host assigned a track
      connectedPeers.update(peers =>
        peers.map(p => p.id === data.peerId ? { ...p, trackId: data.trackId } : p)
      );
      if (data.peerId === peer.id || (data.peerId === 'host' && $isHost)) {
        myTrackId.set(data.trackId);
      }
      break;

    case 'tracks_update':
      availableTracks.set(data.tracks);
      break;

    case 'slot_assign':
      // Host assigned a slot for split mode
      connectedPeers.update(peers =>
        peers.map(p => p.id === data.peerId ? { ...p, slot: data.slot } : p)
      );
      if (data.peerId === peer.id || (data.peerId === 'host' && $isHost)) {
        mySlot.set(data.slot);
      }
      break;

    case 'mode_change':
      bandPlayMode.set(data.mode);
      break;

    case 'play':
      // Synchronized play command
      handlePlayCommand(data);
      break;

    case 'pause':
      handlePauseCommand(data);
      break;

    case 'stop':
      handleStopCommand();
      break;

    case 'seek':
      handleSeekCommand(data);
      break;

    case 'sync':
      // Sync pulse for drift correction
      handleSyncPulse(data);
      break;

    case 'room_closed':
      // Host left, all members must leave
      console.log('Host closed the room');
      handleRoomClosed();
      break;

    case 'ready':
      // Member toggled ready state
      if ($isHost) {
        connectedPeers.update(peers =>
          peers.map(p => p.id === fromPeerId ? { ...p, ready: data.ready } : p)
        );
        // Broadcast updated ready state to all
        broadcast({ type: 'ready_update', peerId: fromPeerId, ready: data.ready }, fromPeerId);
      }
      break;

    case 'ready_update':
      // Ready state broadcast from host
      connectedPeers.update(peers =>
        peers.map(p => p.id === data.peerId ? { ...p, ready: data.ready } : p)
      );
      break;

    case 'song_select':
      // Host selected a song - check if we have it locally
      handleSongSelect(data);
      break;

    case 'song_data':
      // Receive song file data from host
      handleSongData(data);
      break;

    case 'ready_reset':
      // Reset all ready states (on stop)
      handleReadyReset();
      break;

    case 'kicked':
      // We've been kicked by the host
      console.log('You were kicked from the room');
      handleKicked();
      break;

    case 'peer_kicked':
      // Another player was kicked (broadcast from host)
      connectedPeers.update(peers =>
        peers.filter(p => p.id !== data.peerId)
      );
      break;

    case 'settings_sync':
      // Host synced settings - apply them (member only)
      if (!$isHost) {
        handleSettingsSync(data);
      }
      break;

    case 'calibrate_start':
      // Host started calibration
      handleCalibrateStart(data);
      break;

    case 'calibrate_stop':
      // Host stopped calibration
      handleCalibrateStop();
      break;
  }
}

// Latency measurement
function startLatencyMeasurement(peerId, conn) {
  // Measure every 2 seconds
  const interval = setInterval(() => {
    if (conn.open) {
      conn.send({
        type: 'ping',
        timestamp: Date.now()
      });
    }
  }, 2000);

  latencyIntervals.set(peerId, interval);

  // Initial ping
  conn.send({ type: 'ping', timestamp: Date.now() });
}

// Broadcast message to all peers (host only)
function broadcast(data, excludePeerId = null) {
  connections.forEach((conn, peerId) => {
    if (peerId !== excludePeerId && conn.open) {
      conn.send(data);
    }
  });
}

// Send to host (player only)
function sendToHost(data) {
  const hostConn = connections.get('host');
  if (hostConn && hostConn.open) {
    hostConn.send(data);
  }
}

// Set available tracks (host, after loading MIDI)
export function setAvailableTracks(tracks) {
  availableTracks.set(tracks);

  if (get(isHost)) {
    broadcast({ type: 'tracks_update', tracks });
  }
}

// Load tracks from a MIDI file (host only)
export async function loadTracksFromFile(filePath) {
  if (!get(isHost)) return;

  try {
    const tracks = await invoke('get_midi_tracks', { path: filePath });
    setAvailableTracks(tracks);
    return tracks;
  } catch (error) {
    console.error('Failed to load tracks:', error);
    return [];
  }
}

// Start song selection mode
export function startBandSongSelect() {
  bandSongSelectMode.set(true);
}

// Select a song for band mode
export async function selectBandSong(file) {
  if (!get(isHost)) return;

  // Extract filename from path
  const filename = file.path.split(/[\\/]/).pop();

  // Read file as base64 for transfer
  let fileData = null;
  try {
    fileData = await invoke('read_midi_base64', { path: file.path });
  } catch (err) {
    console.error('Failed to read MIDI file:', err);
  }

  // Store with file data for new joiners
  bandSelectedSong.set({ ...file, filename, fileData });
  bandSongSelectMode.set(false);
  bandFilePath.set(file.path);

  // Reset all ready states
  connectedPeers.update(peers =>
    peers.map(p => ({ ...p, ready: p.isHost ? true : false }))
  );

  // Auto-load tracks
  if (file?.path) {
    await loadTracksFromFile(file.path);
  }

  // Broadcast song select to members
  broadcast({ type: 'song_select', name: file.name, filename });

  // Send file data to all members
  if (fileData) {
    broadcast({ type: 'song_data', filename, fileData });
  }
}

// Cancel song selection
export function cancelBandSongSelect() {
  bandSongSelectMode.set(false);
}

// Assign track to player (host only)
export function assignTrack(peerId, trackId) {
  if (!get(isHost)) return;

  connectedPeers.update(peers =>
    peers.map(p => p.id === peerId ? { ...p, trackId } : p)
  );

  if (peerId === 'host') {
    myTrackId.set(trackId);
  }

  broadcast({ type: 'track_assign', peerId, trackId });
}

// Assign slot to player for split mode (host only)
export function assignSlot(peerId, slot) {
  if (!get(isHost)) return;

  connectedPeers.update(peers =>
    peers.map(p => p.id === peerId ? { ...p, slot } : p)
  );

  if (peerId === 'host') {
    mySlot.set(slot);
  }

  broadcast({ type: 'slot_assign', peerId, slot });
}

// Auto-assign slots to all players (host only)
export function autoAssignSlots() {
  if (!get(isHost)) return;

  const peers = get(connectedPeers);
  peers.forEach((peer, index) => {
    assignSlot(peer.id, index);
  });
}

// Broadcast current settings to all members (host only)
// Call this when host changes any setting during band mode
export function broadcastSettings() {
  if (!get(isHost)) return;
  if (!get(bandEnabled)) return;

  const settingsCmd = {
    type: 'settings_sync',
    settings: {
      speed: get(speed),
      noteMode: get(noteMode),
      keyMode: get(keyMode),
      octaveShift: get(octaveShift)
    }
  };

  broadcast(settingsCmd);
  console.log('[BAND] Broadcasting settings to members:', settingsCmd.settings);
}

// Handle settings sync from host (member only)
async function handleSettingsSync(data) {
  const { settings } = data;
  if (!settings) return;

  console.log('[BAND] Received settings sync from host:', settings);
  await setSpeed(settings.speed);
  await setNoteMode(settings.noteMode);
  await setKeyMode(settings.keyMode);
  await setOctaveShift(settings.octaveShift);
}

// Set band play mode (host only)
export function setBandPlayMode(mode) {
  if (!get(isHost)) return;

  bandPlayMode.set(mode);
  broadcast({ type: 'mode_change', mode });

  // Auto-assign slots when switching to split mode
  if (mode === 'split') {
    autoAssignSlots();
  }
}

// Start calibration mode (host only)
// All players will play a repeating note pattern for sync testing
export function startCalibration() {
  if (!get(isHost)) return;

  const peers = get(connectedPeers);
  const maxLatency = Math.max(...peers.map(p => p.latency), 0);
  const buffer = Math.max(maxLatency * 2 + 100, 300);
  const startAt = Date.now() + buffer;

  const calibrateCmd = {
    type: 'calibrate_start',
    startAt,
    interval: 1500 // Play note every 1.5 seconds for easier listening
  };

  broadcast(calibrateCmd);
  handleCalibrateStart(calibrateCmd);
}

// Stop calibration mode (host only)
export function stopCalibration() {
  if (!get(isHost)) return;

  broadcast({ type: 'calibrate_stop' });
  handleCalibrateStop();
}

// Handle calibration start (all peers)
async function handleCalibrateStart(data) {
  const { startAt, interval } = data;
  const $isHost = get(isHost);

  isCalibrating.set(true);

  // Store reference time for calculating note timings
  let noteIndex = 0;
  const calibrationStartTime = startAt;

  function scheduleNextNote() {
    if (!get(isCalibrating)) return;

    const now = Date.now();
    // Host recalculates hostDelay each time so slider changes take effect
    const hostOffset = $isHost ? get(hostDelay) : 0;

    // Each note plays at: startAt + (noteIndex * interval) + hostOffset
    const targetTime = calibrationStartTime + (noteIndex * interval) + hostOffset;
    const delay = targetTime - now;

    if (delay < -interval) {
      // Way behind, skip to catch up
      noteIndex++;
      scheduleNextNote();
      return;
    }

    calibrationInterval = setTimeout(async () => {
      if (!get(isCalibrating)) return;

      await invoke('press_key', { key: 'q' });
      noteIndex++;
      scheduleNextNote();
    }, Math.max(0, delay));
  }

  scheduleNextNote();
  console.log(`[BAND] Calibration started`);
}

// Handle calibration stop (all peers)
function handleCalibrateStop() {
  isCalibrating.set(false);
  if (calibrationInterval) {
    clearTimeout(calibrationInterval);
    calibrationInterval = null;
  }
  console.log('[BAND] Calibration stopped');
}

// Synchronized play (host only)
export function bandPlay(position = 0) {
  if (!get(isHost)) return;

  const peers = get(connectedPeers);
  const maxLatency = Math.max(...peers.map(p => p.latency), 0);
  const mode = get(bandPlayMode);
  const totalPlayers = peers.length;

  // Schedule start time with buffer for highest latency peer
  const buffer = Math.max(maxLatency * 2 + 100, 300); // At least 300ms
  const startAt = Date.now() + buffer;

  // Include all host settings for sync
  const playCmd = {
    type: 'play',
    startAt,
    position,
    mode,
    totalPlayers,
    // Sync all host settings
    settings: {
      speed: get(speed),
      noteMode: get(noteMode),
      keyMode: get(keyMode),
      octaveShift: get(octaveShift)
    }
  };

  broadcast(playCmd);
  handlePlayCommand(playCmd); // Also execute locally
}

// Synchronized pause (host only)
export function bandPause() {
  if (!get(isHost)) return;

  const pauseCmd = { type: 'pause', timestamp: Date.now() };
  broadcast(pauseCmd);
  handlePauseCommand(pauseCmd);
}

// Synchronized stop (host only)
export function bandStop() {
  if (!get(isHost)) return;

  broadcast({ type: 'stop' });
  handleStopCommand();

  // Reset all ready states
  broadcast({ type: 'ready_reset' });
  connectedPeers.update(peers =>
    peers.map(p => ({ ...p, ready: p.isHost ? true : false }))
  );
}

// Toggle ready state (member only)
export function toggleReady() {
  if (get(isHost)) return; // Host is always ready

  const newReady = !get(myReady);
  myReady.set(newReady);

  // Update own entry in connectedPeers immediately for UI feedback
  if (peer && peer.id) {
    connectedPeers.update(peers =>
      peers.map(p => p.id === peer.id ? { ...p, ready: newReady } : p)
    );
  }

  // Send to host
  const hostConn = connections.get('host');
  if (hostConn && hostConn.open) {
    hostConn.send({ type: 'ready', ready: newReady });
  }
}

// Check if all members are ready (for host)
export function allMembersReady() {
  const peers = get(connectedPeers);
  // All non-host peers must be ready
  return peers.filter(p => !p.isHost).every(p => p.ready);
}

// Synchronized seek (host only)
export function bandSeek(position) {
  if (!get(isHost)) return;

  const peers = get(connectedPeers);
  const maxLatency = Math.max(...peers.map(p => p.latency), 0);

  // Schedule seek time with buffer for highest latency peer
  const buffer = Math.max(maxLatency * 2 + 50, 150); // At least 150ms
  const seekAt = Date.now() + buffer;

  const seekCmd = {
    type: 'seek',
    seekAt,
    position,
    // Include settings for sync on seek
    settings: {
      speed: get(speed),
      noteMode: get(noteMode),
      keyMode: get(keyMode),
      octaveShift: get(octaveShift)
    }
  };

  broadcast(seekCmd);
  handleSeekCommand(seekCmd); // Also execute locally
}

// Handle play command (all peers)
async function handlePlayCommand(data) {
  const { startAt, position, mode, totalPlayers, settings } = data;
  const now = Date.now();
  const $isHost = get(isHost);

  // Apply host settings before playing (members only)
  if (!$isHost && settings) {
    console.log('[BAND] Applying host settings:', settings);
    await setSpeed(settings.speed);
    await setNoteMode(settings.noteMode);
    await setKeyMode(settings.keyMode);
    await setOctaveShift(settings.octaveShift);
  }

  // Host adds extra delay to let members receive and process the command
  const hostOffset = $isHost ? get(hostDelay) : 0;
  const delay = (startAt - now) + hostOffset;

  // Get this player's slot
  const $mySlot = get(mySlot) ?? 0;
  const $bandSelectedSong = get(bandSelectedSong);
  const $myTrackId = get(myTrackId);

  console.log(`Play scheduled in ${delay}ms (host offset: ${hostOffset}ms) at position ${position}, mode: ${mode}, slot: ${$mySlot}/${totalPlayers}`);

  // Import player functions
  const { playMidiBand, currentFile, seekTo } = await import('./player.js');

  // Use band selected song or current file
  const fileToPlay = $bandSelectedSong || get(currentFile);

  const playOptions = {
    mode: mode || 'split',
    slot: $mySlot,
    totalPlayers: totalPlayers || 1,
    trackId: $myTrackId
  };

  if (delay > 0) {
    setTimeout(async () => {
      if (fileToPlay) {
        await seekTo(position);
        await playMidiBand(fileToPlay, playOptions);
      }
    }, delay);
  } else {
    // Already past start time, play immediately
    if (fileToPlay) {
      await seekTo(position);
      await playMidiBand(fileToPlay, playOptions);
    }
  }
}

// Handle pause command
async function handlePauseCommand(data) {
  const { pauseResume, isPaused } = await import('./player.js');
  const $isPaused = get(isPaused);

  if (!$isPaused) {
    await pauseResume();
  }
}

// Handle stop command
async function handleStopCommand() {
  const { stopPlayback } = await import('./player.js');
  await stopPlayback();

  // Reset ready state for all (local only, host broadcasts separately)
  myReady.set(false);
}

// Handle song select from host (member side)
async function handleSongSelect(data) {
  const { filename, name } = data;

  // Reset ready state when song changes
  myReady.set(false);

  // Check if we have this file locally
  const localPath = await invoke('check_midi_exists', { filename });

  if (localPath) {
    // We have the file locally
    console.log('Using local file:', localPath);
    bandFilePath.set(localPath);
    bandSelectedSong.set({ name, filename, path: localPath });

    // Auto-ready if enabled
    if (get(autoReady)) {
      myReady.set(true);
      // Notify host
      const hostConn = connections.get('host');
      if (hostConn && hostConn.open) {
        hostConn.send({ type: 'ready', ready: true });
      }
      console.log('[BAND] Auto-ready: file found locally');
    }
  } else {
    // We don't have the file - wait for song_data
    console.log('File not found locally, waiting for transfer:', filename);
    bandFilePath.set(null);
    bandSelectedSong.set({ name, filename, path: null, pending: true });
  }
}

// Handle ready reset (on stop) - re-apply auto-ready if enabled
function handleReadyReset() {
  const $isHost = get(isHost);

  // Reset ready states
  connectedPeers.update(peers =>
    peers.map(p => ({ ...p, ready: p.isHost ? true : false }))
  );

  // For members: check auto-ready
  if (!$isHost && get(autoReady) && get(bandFilePath)) {
    // Auto-ready since we have the song file
    myReady.set(true);
    const hostConn = connections.get('host');
    if (hostConn && hostConn.open) {
      hostConn.send({ type: 'ready', ready: true });
    }
    console.log('[BAND] Auto-ready after stop');
  } else {
    myReady.set(false);
  }
}

// Handle song data transfer from host (member side)
async function handleSongData(data) {
  const { filename, fileData } = data;

  try {
    // Save to temp
    const tempPath = await invoke('save_temp_midi', { filename, dataBase64: fileData });
    console.log('Saved temp file:', tempPath);

    // Update state
    bandFilePath.set(tempPath);
    bandSelectedSong.update(song => song ? { ...song, path: tempPath, pending: false } : null);

    // Auto-ready if enabled
    if (get(autoReady)) {
      myReady.set(true);
      // Notify host
      const hostConn = connections.get('host');
      if (hostConn && hostConn.open) {
        hostConn.send({ type: 'ready', ready: true });
      }
      console.log('[BAND] Auto-ready: file received');
    }
  } catch (err) {
    console.error('Failed to save temp MIDI:', err);
  }
}

// Handle seek command (all peers)
async function handleSeekCommand(data) {
  const { seekAt, position, settings } = data;
  const now = Date.now();
  const delay = seekAt - now;
  const $isHost = get(isHost);

  // Apply host settings on seek (members only)
  if (!$isHost && settings) {
    console.log('[BAND] Applying host settings on seek:', settings);
    await setSpeed(settings.speed);
    await setNoteMode(settings.noteMode);
    await setKeyMode(settings.keyMode);
    await setOctaveShift(settings.octaveShift);
  }

  const { seekTo } = await import('./player.js');

  if (delay > 0) {
    // Wait until scheduled time for sync
    setTimeout(async () => {
      await seekTo(position);
    }, delay);
  } else {
    // Already past seek time, execute immediately
    await seekTo(position);
  }
}

// Handle sync pulse for drift correction
async function handleSyncPulse(data) {
  // TODO: Implement drift correction
  // Compare data.position with our current position
  // Nudge playback if difference > threshold
}

// Start sync pulse (host only, call during playback)
export function startSyncPulse() {
  if (!get(isHost)) return;

  // Send sync every 5 seconds
  syncInterval = setInterval(async () => {
    const { currentPosition } = await import('./player.js');
    const position = get(currentPosition);

    broadcast({
      type: 'sync',
      position,
      timestamp: Date.now()
    });
  }, 5000);
}

export function stopSyncPulse() {
  if (syncInterval) {
    clearInterval(syncInterval);
    syncInterval = null;
  }
}

// Handle room closed by host (for members)
async function handleRoomClosed() {
  // Stop any playback
  const { stopPlayback } = await import('./player.js');
  await stopPlayback();

  // Restore original settings before leaving
  await restoreOriginalSettings();

  // Clean up without broadcasting (we're receiving, not sending)
  stopSyncPulse();

  latencyIntervals.forEach(interval => clearInterval(interval));
  latencyIntervals.clear();

  connections.forEach(conn => conn.close());
  connections.clear();

  if (peer) {
    peer.destroy();
    peer = null;
  }

  // Reset state
  bandEnabled.set(false);
  isHost.set(false);
  roomCode.set(null);
  connectedPeers.set([]);
  myTrackId.set(null);
  mySlot.set(null);
  availableTracks.set([]);
  bandStatus.set('disconnected');
  bandPlayMode.set('split');
  bandSelectedSong.set(null);
}

// Handle being kicked (for members)
async function handleKicked() {
  // Stop any playback
  const { stopPlayback } = await import('./player.js');
  await stopPlayback();

  // Restore original settings before leaving
  await restoreOriginalSettings();

  // Clean up without broadcasting
  stopSyncPulse();

  latencyIntervals.forEach(interval => clearInterval(interval));
  latencyIntervals.clear();

  connections.forEach(conn => conn.close());
  connections.clear();

  if (peer) {
    peer.destroy();
    peer = null;
  }

  // Reset state
  bandEnabled.set(false);
  isHost.set(false);
  roomCode.set(null);
  connectedPeers.set([]);
  myTrackId.set(null);
  mySlot.set(null);
  availableTracks.set([]);
  bandStatus.set('disconnected');
  bandPlayMode.set('split');
  bandSelectedSong.set(null);
  myReady.set(false);
}

// Kick a player from the room (host only)
export function kickPlayer(peerId) {
  if (!get(isHost)) return;
  if (peerId === 'host') return; // Can't kick yourself

  const conn = connections.get(peerId);
  if (conn && conn.open) {
    // Send kick message to the player
    conn.send({ type: 'kicked' });

    // Close their connection
    setTimeout(() => {
      conn.close();
    }, 100);
  }

  // Remove from connections
  connections.delete(peerId);

  // Clear latency interval
  const interval = latencyIntervals.get(peerId);
  if (interval) {
    clearInterval(interval);
    latencyIntervals.delete(peerId);
  }

  // Remove from connected peers
  connectedPeers.update(peers =>
    peers.filter(p => p.id !== peerId)
  );

  // Notify other players
  broadcast({ type: 'peer_kicked', peerId });
}

// Leave room / cleanup
export async function leaveRoom() {
  const $isHost = get(isHost);

  // If host is leaving, notify all members first
  if ($isHost) {
    broadcast({ type: 'room_closed' });
  } else {
    // Member leaving - restore original settings
    await restoreOriginalSettings();
  }

  stopSyncPulse();

  latencyIntervals.forEach(interval => clearInterval(interval));
  latencyIntervals.clear();

  connections.forEach(conn => conn.close());
  connections.clear();

  if (peer) {
    peer.destroy();
    peer = null;
  }

  // Reset state
  bandEnabled.set(false);
  isHost.set(false);
  roomCode.set(null);
  connectedPeers.set([]);
  myTrackId.set(null);
  mySlot.set(null);
  availableTracks.set([]);
  bandStatus.set('disconnected');
  bandPlayMode.set('split');
  bandSelectedSong.set(null);
}

// Toggle band mode
export function toggleBandMode() {
  const enabled = !get(bandEnabled);
  bandEnabled.set(enabled);

  if (!enabled) {
    leaveRoom();
  }
}
