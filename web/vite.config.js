import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

export default defineConfig({
    plugins: [
        wasm(),
        topLevelAwait()
    ],
    base: "/hades2-tools"
});