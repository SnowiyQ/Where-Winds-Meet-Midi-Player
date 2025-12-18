import '@testing-library/jest-dom';
import { vi } from 'vitest';

vi.mock('./lib/tauri/core-proxy.js', () => ({
	invoke: () => Promise.resolve(null),
}))

const storage = new Map();
globalThis.localStorage = {
	getItem(key) {
		return storage.has(key) ? storage.get(key) : null;
	},
	setItem(key, value) {
		storage.set(key, String(value));
	},
	removeItem(key) {
		storage.delete(key);
	},
	clear() {
		storage.clear();
	},
};
