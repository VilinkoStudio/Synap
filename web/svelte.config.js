import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  compilerOptions: {
    experimental: {
      async: true
    }
  },
  kit: {
    adapter: adapter(),
    alias: {
      $lib: './src/lib'
    },
    csrf: {
      trustedOrigins: ['http://localhost:5173', 'http://127.0.0.1:5173']
    },
    experimental: {
      remoteFunctions: true
    }
  }
};

export default config;
