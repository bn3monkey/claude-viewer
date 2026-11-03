import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 가 띄우는 개발 서버. 포트는 tauri.conf.json 의 devUrl 과 맞춘다.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { target: "es2022", chunkSizeWarningLimit: 4096 },
});
