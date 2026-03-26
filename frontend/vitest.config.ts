import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["**/*.test.{ts,tsx}"],
    server: {
      deps: {
        inline: ["lucide-react"],
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "."),
      "lucide-react": path.resolve(__dirname, "__mocks__/lucide-react.tsx"),
    },
  },
});
