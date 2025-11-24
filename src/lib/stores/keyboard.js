import { writable } from 'svelte/store';

// Keyboard layout
export const LOW_KEYS = ['z', 'x', 'c', 'v', 'b', 'n', 'm'];
export const MID_KEYS = ['a', 's', 'd', 'f', 'g', 'h', 'j'];
export const HIGH_KEYS = ['q', 'w', 'e', 'r', 't', 'y', 'u'];

// Active keys state
export const activeKeys = writable(new Set());

// Add key as active
export function activateKey(key) {
  activeKeys.update(keys => {
    keys.add(key.toLowerCase());
    return new Set(keys);
  });
}

// Remove key from active
export function deactivateKey(key) {
  activeKeys.update(keys => {
    keys.delete(key.toLowerCase());
    return new Set(keys);
  });
}

// Clear all active keys
export function clearActiveKeys() {
  activeKeys.set(new Set());
}