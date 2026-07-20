import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const appDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(appDir, "../..");
const workspaceNodeModules = path.join(workspaceRoot, "node_modules");

export default defineConfig({
  plugins: [react()],
  cacheDir: path.join(workspaceRoot, "node_modules/.vite/apps-desktop"),
  resolve: {
    alias: {
      react: path.join(workspaceNodeModules, "react"),
      "react-dom": path.join(workspaceNodeModules, "react-dom"),
      "react-dom/client": path.join(workspaceNodeModules, "react-dom/client"),
      "react/jsx-runtime": path.join(workspaceNodeModules, "react/jsx-runtime"),
      "@tanstack/react-query": path.join(workspaceNodeModules, "@tanstack/react-query"),
      "@tanstack/query-core": path.join(workspaceNodeModules, "@tanstack/query-core"),
    },
    dedupe: ["react", "react-dom", "@tanstack/react-query", "@tanstack/query-core"],
  },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: false,
  },
  optimizeDeps: {
    include: [
      "@tanstack/react-query",
      "react",
      "react-dom",
      "react-dom/client",
      "react/jsx-dev-runtime",
      "react/jsx-runtime",
    ],
  },
  test: {
    environment: "node",
    setupFiles: ["src/test/setup.ts"],
    globals: true,
    pool: "threads",
    maxWorkers: 1,
    minWorkers: 1,
    fileParallelism: false,
    exclude: ["tests/**", "node_modules/**", "dist/**"],
  },
});
