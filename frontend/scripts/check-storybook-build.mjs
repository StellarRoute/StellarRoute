/**
 * check-storybook-build.mjs
 *
 * Verifies the Ladle production build produced expected artifacts.
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BUILD_DIR = path.resolve(__dirname, "..", "build");
const MAX_BUILD_BYTES = 2 * 1024 * 1024; // 2 MiB guardrail

function directorySizeBytes(dir) {
  let total = 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      total += directorySizeBytes(fullPath);
    } else if (entry.isFile()) {
      total += fs.statSync(fullPath).size;
    }
  }
  return total;
}

if (!fs.existsSync(BUILD_DIR)) {
  console.error(
    `[storybook:ci] ERROR: Build output not found at "${BUILD_DIR}". Run "ladle build" first.`
  );
  process.exit(1);
}

const indexHtml = path.join(BUILD_DIR, "index.html");
if (!fs.existsSync(indexHtml)) {
  console.error(`[storybook:ci] ERROR: Missing ${indexHtml}`);
  process.exit(1);
}

const bytes = directorySizeBytes(BUILD_DIR);
const kb = bytes / 1024;

if (bytes > MAX_BUILD_BYTES) {
  console.error(
    `[storybook:ci] ERROR: Build size ${kb.toFixed(1)} KB exceeds ${MAX_BUILD_BYTES / 1024} KB limit.`
  );
  process.exit(1);
}

console.log(`[storybook:ci] Build verified — ${kb.toFixed(1)} KB at ${BUILD_DIR}`);
