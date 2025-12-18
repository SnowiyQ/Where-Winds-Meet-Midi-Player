export function formatTime(seconds) {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

export function calculateProgress(position, duration) {
  if (duration === 0) return 0;
  if (position <= 0) return 0;
  if (position >= duration) return 100;
  return (position / duration) * 100;
}

export function clampSpeed(speed) {
  const min = 0.25;
  const max = 2;
  if (Number.isNaN(Number(speed))) return min;
  return Math.min(Math.max(Number(speed), min), max);
}

export function describePlaybackState({ isPlaying, isPaused }) {
  if (isPlaying && !isPaused) return 'playing';
  if (isPlaying && isPaused) return 'paused';
  return 'stopped';
}
