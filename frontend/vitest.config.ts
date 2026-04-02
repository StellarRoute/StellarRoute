import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["**/*.test.{ts,tsx}"],
    setupFiles: ["./vitest.setup.ts"],
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "."),
      "lucide-react": path.resolve(__dirname, "__mocks__/lucide-react.tsx"),
      "@stellar/freighter-api": path.resolve(__dirname, "__mocks__/@stellar/freighter-api.ts"),
    },
  },
});
