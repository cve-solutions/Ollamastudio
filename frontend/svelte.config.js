import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({ port: 3000 }),
    csrf: {
      checkOrigin: false, // Les requêtes /api/* sont proxifiées vers le backend
    },
    alias: {
      $lib: './src/lib',
      $api: './src/lib/api',
      $stores: './src/lib/stores',
    },
  },
};

export default config;
