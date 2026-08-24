import path from 'node:path';
import { fileURLToPath } from 'node:url';

const dirname = path.dirname(fileURLToPath(import.meta.url));

/** @type {import('@ladle/react').UserConfig} */
export default {
  stories: 'components/**/*.stories.{tsx,jsx,ts,js}',
  viteConfig: path.resolve(dirname, '..', 'ladle-vite.config.ts'),
};
