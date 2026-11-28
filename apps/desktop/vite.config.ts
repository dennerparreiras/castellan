import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The Tauri window loads this bundle from its internal protocol. Nothing is
// fetched from the network at runtime, so there is no CDN and no remote font.
export default defineConfig({
    plugins: [react()],
    clearScreen: false,
    server: {
        port: 5173,
        strictPort: true,
        host: "localhost",
    },
    build: {
        target: "es2022",
        sourcemap: false,
        // Keep the CSP simple: no inlined scripts, assets as files.
        assetsInlineLimit: 0,
    },
    test: {
        environment: "jsdom",
        setupFiles: ["./src/test-setup.ts"],
        restoreMocks: true,
    },
});
