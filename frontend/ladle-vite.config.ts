import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  resolve: {
    alias: {
      '@': dirname,
      'next/navigation': path.resolve(dirname, '__mocks__/next-navigation.ts'),
      'next/dynamic': path.resolve(dirname, '__mocks__/next-dynamic.tsx'),
      '@stellar/freighter-api': path.resolve(
        dirname,
        '__mocks__/@stellar/freighter-api.browser.ts',
      ),
    },
  },
});
