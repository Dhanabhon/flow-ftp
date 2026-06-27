import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      // Tauri expects a SPA build (no SSR, no prerender of dynamic routes).
      fallback: 'index.html',
      strict: false
    })
  }
};

export default config;
