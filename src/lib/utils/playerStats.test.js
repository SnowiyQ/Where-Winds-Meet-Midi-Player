import { describe, expect, it } from 'vitest'
import { clampSpeed, calculateProgress, describePlaybackState, formatTime } from './playerStats.js'

describe('playerStats helpers', () => {
  it('formats seconds into minutes:seconds', () => {
    expect(formatTime(0)).toBe('0:00')
    expect(formatTime(65)).toBe('1:05')
    expect(formatTime(125)).toBe('2:05')
  })

  it('calculates progress percent safely', () => {
    expect(calculateProgress(0, 0)).toBe(0)
    expect(calculateProgress(5, 10)).toBe(50)
    expect(calculateProgress(25, 10)).toBe(100)
    expect(calculateProgress(-5, 10)).toBe(0)
  })

  it('clamps speeds within supported range', () => {
    expect(clampSpeed(0.1)).toBe(0.25)
    expect(clampSpeed(1.5)).toBe(1.5)
    expect(clampSpeed(10)).toBe(2)
    expect(clampSpeed('abc')).toBe(0.25)
  })

  it('describes playback states consistently', () => {
    expect(describePlaybackState({ isPlaying: true, isPaused: false })).toBe('playing')
    expect(describePlaybackState({ isPlaying: true, isPaused: true })).toBe('paused')
    expect(describePlaybackState({ isPlaying: false, isPaused: false })).toBe('stopped')
  })
})
