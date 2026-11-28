import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const apiTarget = process.env.VITE_API_TARGET || "http://127.0.0.1:3000";
const allowedHosts = [".trycloudflare.com"];

const proxy = {
  "/v1": {
    target: apiTarget,
    changeOrigin: true,
  },
  "/health": {
    target: apiTarget,
    changeOrigin: true,
  },
};

export default defineConfig({
  plugins: [react()],
  server: {
    allowedHosts,
    proxy,
  },
  preview: {
    allowedHosts,
    proxy,
  },
  build: {
    target: "es2022",
    sourcemap: false,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
  },
});
