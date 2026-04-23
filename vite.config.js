"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const vite_1 = require("vite");
const path_1 = require("path");
const fs_1 = require("fs");
// Plugin to preserve theme injection placeholders and move HTML to dist root
function preservePlaceholders() {
    return {
        name: "preserve-placeholders",
        transformIndexHtml: {
            order: "post",
            handler(html) {
                // Ensure placeholders remain in final output
                if (!html.includes("<!--HEAD-->")) {
                    html = html.replace("</head>", "<!--HEAD-->\n</head>");
                }
                if (!html.includes("<!--BODY-->")) {
                    html = html.replace("<body>", "<body>\n<!--BODY-->");
                }
                return html;
            },
        },
        closeBundle() {
            // Move app.html from dist/frontend/ to dist/
            const src = "dist/frontend/index.html";
            const dest = "dist/index.html";
            (0, fs_1.copyFileSync)(src, dest);
            // Remove the frontend directory
            (0, fs_1.rmSync)("dist/frontend", { recursive: true, force: true });
        },
    };
}
exports.default = (0, vite_1.defineConfig)({
    plugins: [preservePlaceholders()],
    base: "/dist/",
    build: {
        outDir: "dist",
        target: "es2020",
        emptyOutDir: true,
        reportCompressedSize: process.env.NODE_ENV === "production",
        minify: process.env.NODE_ENV === "production",
        modulePreload: {
            polyfill: false,
        },
        rollupOptions: {
            input: {
                app: (0, path_1.resolve)(__dirname, "frontend/index.html"),
            },
            output: {
                entryFileNames: "main.[hash].js",
                chunkFileNames: "main.[hash].js",
                assetFileNames: (assetInfo) => {
                    if (assetInfo.name?.endsWith(".css")) {
                        return "main.[hash].css";
                    }
                    return "assets/[name]-[hash][extname]";
                },
            },
        },
    },
});
