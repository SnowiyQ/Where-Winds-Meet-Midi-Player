import { register, init, locale, waitLocale } from 'svelte-i18n';
import { writable, derived } from 'svelte/store';
import { invoke } from '../tauri/core-proxy.js';

// Storage key for persistence
const STORAGE_KEY = 'wwm-language';

// Available languages (flags use Iconify circle-flags)
export const languages = [
  { code: 'en', name: 'English', flag: 'circle-flags:us' },
  { code: 'zh', name: '中文', flag: 'circle-flags:cn' },
  { code: 'ja', name: '日本語', flag: 'circle-flags:jp' },
  { code: 'ko', name: '한국어', flag: 'circle-flags:kr' },
  { code: 'th', name: 'ไทย', flag: 'circle-flags:th' },
];

// Deep merge two objects (user values override defaults)
function deepMerge(target, source) {
  const result = { ...target };
  for (const key in source) {
    if (source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])) {
      result[key] = deepMerge(target[key] || {}, source[key]);
    } else {
      result[key] = source[key];
    }
  }
  return result;
}

// Check if source has keys that target doesn't have
function hasNewKeys(bundled, user, path = '') {
  for (const key in bundled) {
    if (!(key in user)) {
      return true;
    }
    if (bundled[key] && typeof bundled[key] === 'object' && !Array.isArray(bundled[key])) {
      if (!user[key] || typeof user[key] !== 'object') {
        return true;
      }
      if (hasNewKeys(bundled[key], user[key], `${path}.${key}`)) {
        return true;
      }
    }
  }
  return false;
}

// Bundled locale imports
const bundledLocales = {
  en: () => import('./locales/en.json'),
  zh: () => import('./locales/zh.json'),
  ja: () => import('./locales/ja.json'),
  ko: () => import('./locales/ko.json'),
  th: () => import('./locales/th.json'),
};

// Load locale with user customizations merged
async function loadLocaleWithUserOverrides(lang) {
  // Load bundled locale first
  const bundledModule = await bundledLocales[lang]();
  const bundled = bundledModule.default || bundledModule;

  try {
    // Try to load user locale from file system
    const userLocale = await invoke('get_user_locale', { lang });

    if (userLocale) {
      // Check if bundled has new keys that user file doesn't have
      if (hasNewKeys(bundled, userLocale)) {
        // Merge: bundled as base, user values override
        const merged = deepMerge(bundled, userLocale);
        // Save merged result back to user file (adds new keys)
        try {
          await invoke('save_user_locale', { lang, data: merged });
          console.log(`Updated user locale ${lang} with new keys from app update`);
        } catch (saveErr) {
          console.warn(`Failed to save updated locale for ${lang}:`, saveErr);
        }
        return merged;
      }

      // No new keys, just use user's file as-is (it has all keys)
      return userLocale;
    }
  } catch (err) {
    console.warn(`Failed to load user locale for ${lang}:`, err);
  }

  return bundled;
}

// Register locales with custom loader
for (const lang of Object.keys(bundledLocales)) {
  register(lang, () => loadLocaleWithUserOverrides(lang));
}

// Get saved language or default to English
function getInitialLocale() {
  if (typeof localStorage !== 'undefined') {
    return localStorage.getItem(STORAGE_KEY) || 'en';
  }
  return 'en';
}

// Current language store (for UI binding)
export const currentLanguage = writable(getInitialLocale());

// Subscribe to save preference
currentLanguage.subscribe(lang => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, lang);
  }
  locale.set(lang);
});

// Change language function
export function setLanguage(lang) {
  currentLanguage.set(lang);
}

// Get current language info
export const currentLanguageInfo = derived(currentLanguage, ($lang) => {
  return languages.find(l => l.code === $lang) || languages[0];
});

// Initialize user locale files (call on app start)
export async function initUserLocales() {
  try {
    // Load all bundled locales
    const defaultLocales = {};
    for (const [lang, loader] of Object.entries(bundledLocales)) {
      const module = await loader();
      defaultLocales[lang] = module.default || module;
    }

    // Initialize user locale files (creates missing ones)
    await invoke('init_user_locales', { defaultLocales });
    console.log('User locale files initialized');
  } catch (err) {
    console.warn('Failed to initialize user locales:', err);
  }
}

// Open locales folder in file explorer
export async function openLocalesFolder() {
  try {
    await invoke('open_locales_folder');
  } catch (err) {
    console.error('Failed to open locales folder:', err);
  }
}

// Get path to locales folder
export async function getLocalesPath() {
  try {
    return await invoke('get_locales_path');
  } catch (err) {
    console.error('Failed to get locales path:', err);
    return null;
  }
}

// Reload current locale (after user edits)
export async function reloadCurrentLocale() {
  const lang = localStorage.getItem(STORAGE_KEY) || 'en';
  // Force reload by setting to null then back
  locale.set(null);
  await waitLocale();
  locale.set(lang);
  await waitLocale();
  console.log(`Reloaded locale: ${lang}`);
}

// Initialize i18n
init({
  fallbackLocale: 'en',
  initialLocale: getInitialLocale(),
});

export { deepMerge, hasNewKeys, loadLocaleWithUserOverrides };
// Export for convenience
export { t, locale, waitLocale } from 'svelte-i18n';
