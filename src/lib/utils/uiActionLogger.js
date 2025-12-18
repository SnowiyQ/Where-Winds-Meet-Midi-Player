import { writable } from 'svelte/store';

const MAX_LOG_ENTRIES = 250;

function generateId() {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function summarizeValue(value) {
  if (value === undefined) return 'undefined';
  if (value === null) return null;
  if (typeof value === 'function') return `Function(${value.name || 'anonymous'})`;
  if (typeof value !== 'object') return value;
  if (Array.isArray(value)) {
    return `Array(${value.length})`;
  }
  const keys = Object.keys(value);
  try {
    return JSON.parse(JSON.stringify(value));
  } catch {
    return `${value.constructor?.name || 'Object'}(${keys.length})`;
  }
}

function sanitizeMetadata(metadata) {
  if (!metadata || typeof metadata !== 'object') {
    return metadata;
  }
  return Object.fromEntries(
    Object.entries(metadata).map(([key, value]) => [key, summarizeValue(value)])
  );
}

function pushLog(entry) {
  uiActionLog.update((log) => {
    const next = [...log, entry];
    if (next.length > MAX_LOG_ENTRIES) {
      next.shift();
    }
    return next;
  });
}

export const uiActionLog = writable([]);

export function clearUiActionLog() {
  uiActionLog.set([]);
}

export function logUiAction(actionName, stage = 'info', metadata = {}) {
  const entry = {
    id: generateId(),
    actionName,
    stage,
    metadata: sanitizeMetadata(metadata),
    timestamp: new Date().toISOString()
  };
  pushLog(entry);

  const consoleFn =
    stage === 'error' ? console.error : stage === 'warn' ? console.warn : stage === 'completed' ? console.info : console.debug;
  consoleFn(`[UI ACTION] ${actionName} (${stage})`, metadata);
  return entry;
}
