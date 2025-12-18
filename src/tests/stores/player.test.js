import { beforeEach, describe, expect, it, vi } from 'vitest'
import { get } from 'svelte/store'

vi.mock('../../lib/tauri/core-proxy.js', () => ({
  invoke: vi.fn(() => Promise.resolve({})),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve({ unlisten: vi.fn() })),
}))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    innerSize: vi.fn(() => Promise.resolve({ width: 100, height: 100 })),
    innerPosition: vi.fn(() => Promise.resolve({ x: 0, y: 0 })),
    setMinSize: vi.fn(() => Promise.resolve()),
    setSize: vi.fn(() => Promise.resolve()),
  })),
  LogicalSize: class LogicalSize {
    constructor(width = 0, height = 0) {
      this.width = width
      this.height = height
    }
  },
}))

import { invoke } from '../../lib/tauri/core-proxy.js'

import {
  addPlaytime,
  addToSavedPlaylist,
  clearAllFavorites,
  createPlaylist,
  deletePlaylist,
  favorites,
  incrementSession,
  isFavorite,
  reorderPlaylists,
  reorderSavedPlaylist,
  removeDeletedFile,
  removeFromSavedPlaylist,
  savedPlaylists,
  setPlaylistsOrder,
  setPlaylistTracks,
  shuffleMode,
  stats,
  syncFavoritesWithLibrary,
  syncPlaylistsWithLibrary,
  toggleFavorite,
  toggleShuffle,
  trackSongPlay,
} from '../../lib/stores/player.js'

describe('player store helpers', () => {
  const initialStats = {
    totalPlaytime: 0,
    songsPlayed: 0,
    sessionsCount: 0,
    mostPlayed: {},
    lastPlayed: null,
    firstUsed: null,
  }

  let storageSpy

  beforeEach(() => {
    localStorage.clear()
    storageSpy = vi.spyOn(localStorage, 'setItem')
    stats.set({ ...initialStats })
    favorites.set([])
    savedPlaylists.set([])
    shuffleMode.set(false)
    invoke.mockReset()
  })

  afterEach(() => {
    storageSpy.mockRestore()
  })

  it('records plays and persists stats', () => {
    trackSongPlay('demo.mid')
    const recorded = get(stats)
    expect(recorded.songsPlayed).toBe(1)
    expect(recorded.mostPlayed).toHaveProperty('demo.mid', 1)
    expect(storageSpy).toHaveBeenCalledWith(
      'wwm-stats',
      expect.stringContaining('"songsPlayed":1')
    )
  })

  it('accumulates playtime and session counts', () => {
    addPlaytime(45)
    expect(get(stats).totalPlaytime).toBe(45)
    incrementSession()
    expect(get(stats).sessionsCount).toBe(1)
  })

  it('toggles shuffle mode', () => {
    toggleShuffle()
    expect(get(shuffleMode)).toBe(true)
    toggleShuffle()
    expect(get(shuffleMode)).toBe(false)
  })

  it('adds and removes favorites', () => {
    const track = { hash: 'fav-1', name: 'Favorite' }
    toggleFavorite(track)
    expect(get(favorites)).toEqual([track])
    expect(isFavorite('fav-1')).toBe(true)
    toggleFavorite(track)
    expect(get(favorites)).toHaveLength(0)
    clearAllFavorites()
    expect(get(favorites)).toHaveLength(0)
  })

  it('syncs favorites with library metadata', () => {
    favorites.set([{ hash: 'fav-1', name: 'Favorite' }])
    syncFavoritesWithLibrary([{ hash: 'fav-1', name: 'Favorite', path: '/tmp/fav.mid' }])
    expect(get(favorites)[0]).toHaveProperty('path', '/tmp/fav.mid')
  })

  it('creates and mutates playlists', () => {
    const id = createPlaylist('Test')
    const stored = get(savedPlaylists)
    expect(stored).toHaveLength(1)
    expect(stored[0]).toMatchObject({ id, name: 'Test' })

    const track = { hash: 't-1', name: 'Track' }
    addToSavedPlaylist(id, track)
    expect(get(savedPlaylists)[0].tracks).toEqual([track])

    addToSavedPlaylist(id, track)
    expect(get(savedPlaylists)[0].tracks).toHaveLength(1)

    removeFromSavedPlaylist(id, track.hash)
    expect(get(savedPlaylists)[0].tracks).toHaveLength(0)

    deletePlaylist(id)
    expect(get(savedPlaylists)).toHaveLength(0)
  })

  it('reorders playlists and track lists', () => {
    savedPlaylists.set([
      { id: 'a', name: 'A', tracks: [{ hash: '1' }, { hash: '2' }] },
      { id: 'b', name: 'B', tracks: [] },
    ])
    reorderSavedPlaylist('a', 0, 1)
    expect(get(savedPlaylists)[0].tracks.map(t => t.hash)).toEqual(['2', '1'])

    reorderPlaylists(0, 1)
    expect(get(savedPlaylists)[0].id).toBe('b')

    setPlaylistsOrder([{ id: 'c', name: 'C', tracks: [] }])
    expect(get(savedPlaylists)).toHaveLength(1)
    expect(get(savedPlaylists)[0].id).toBe('c')
  })

  it('sets playlist tracks directly', () => {
    savedPlaylists.set([{ id: 'list', name: 'List', tracks: [] }])
    setPlaylistTracks('list', [{ hash: 'alpha' }])
    expect(get(savedPlaylists)[0].tracks).toEqual([{ hash: 'alpha' }])
  })

  it('syncs playlists with library data and handles deletions', () => {
    savedPlaylists.set([
      {
        id: 'list',
        name: 'List',
        tracks: [
          { hash: 'share', name: 'Shared' },
          { hash: 'gone', name: 'Gone' },
        ],
      },
    ])
    syncPlaylistsWithLibrary([{ hash: 'share', name: 'Shared', path: '/music/share.mid' }])
    expect(get(savedPlaylists)[0].tracks.find(t => t.hash === 'share')).toHaveProperty('path', '/music/share.mid')

    favorites.set([{ hash: 'gone' }, { hash: 'share' }])
    removeDeletedFile('gone')
    expect(get(favorites).every(f => f.hash !== 'gone')).toBe(true)
    expect(get(savedPlaylists)[0].tracks.some(t => t.hash === 'gone')).toBe(false)
  })
})
