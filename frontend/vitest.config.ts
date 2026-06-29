import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  test: {
    environment: "jsdom",
    include: [
      "app/**/*.test.{ts,tsx}",
      "components/**/*.test.{ts,tsx}",
      "hooks/**/*.test.{ts,tsx}",
      "lib/**/*.test.{ts,tsx}",
    ],
    exclude: [
      "**/node_modules/**",
      "**/dist/**",
      "**/.next/**",
      "**/account-switcher.test.tsx",
    ],
    setupFiles: ["./vitest.setup.ts"],
    maxWorkers: 2,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "."),
      "lucide-react": path.resolve(__dirname, "__mocks__/lucide-react.tsx"),
      "@stellar/freighter-api": path.resolve(__dirname, "__mocks__/@stellar/freighter-api.ts"),
    },
  },
});
