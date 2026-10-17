/**
 * Theme store — Svelte 5 runes, persisted, follows system preference.
 *
 * Flow:
 *   1. app.html runs a tiny inline script before paint that sets the `dark`
 *      class on <html> from localStorage (or system). This prevents FOUC.
 *   2. This store hydrates from the same source on mount and keeps the
 *      <html> class + localStorage in sync thereafter.
 */
export type ThemeMode = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'flowftp:theme';

class ThemeStore {
  /** What the user explicitly chose. 'system' = follow OS. */
  mode = $state<ThemeMode>('system');
  /** Resolved theme actually applied to <html>. */
  resolved = $state<'light' | 'dark'>('dark');

  #mql: MediaQueryList | null = null;

  /** Hydrate from storage / DOM. Call once on mount. */
  init() {
    if (typeof window === 'undefined') return;

    // Sync from whatever the FOUC-prevention script already set on <html>.
    const htmlDark = document.documentElement.classList.contains('dark');
    this.resolved = htmlDark ? 'dark' : 'light';

    const stored = (localStorage.getItem(STORAGE_KEY) as ThemeMode | null) ?? 'system';
    this.mode = stored;

    // React to OS theme changes when in 'system' mode.
    this.#mql = window.matchMedia('(prefers-color-scheme: dark)');
    this.#mql.addEventListener('change', this.#onSystemChange);
  }

  /** Set explicit mode, apply, persist. */
  setMode(mode: ThemeMode) {
    this.mode = mode;
    localStorage.setItem(STORAGE_KEY, mode);
    this.#apply();
  }

  /** Convenience: flip between the two concrete themes. */
  toggle() {
    this.setMode(this.resolved === 'dark' ? 'light' : 'dark');
  }

  #onSystemChange = (e: MediaQueryListEvent) => {
    if (this.mode === 'system') {
      this.resolved = e.matches ? 'dark' : 'light';
      this.#writeClass();
    }
  };

  /** Compute resolved theme from mode + system, then write <html> class. */
  #apply() {
    const sysDark = this.#mql?.matches ?? false;
    this.resolved =
      this.mode === 'system' ? (sysDark ? 'dark' : 'light') : this.mode;
    this.#writeClass();
  }

  #writeClass() {
    const el = document.documentElement;
    el.classList.toggle('dark', this.resolved === 'dark');
  }
}

export const theme = new ThemeStore();
