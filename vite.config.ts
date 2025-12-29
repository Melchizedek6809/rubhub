import { defineConfig, Plugin } from "vite";
import { resolve } from "path";
import { copyFileSync, rmSync } from "fs";

// Plugin to preserve theme injection placeholders and move HTML to dist root
function preservePlaceholders(): Plugin {
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
			copyFileSync(src, dest);
			// Remove the frontend directory
			rmSync("dist/frontend", { recursive: true, force: true });
		},
	};
}

export default defineConfig({
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
				app: resolve(__dirname, "frontend/index.html"),
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
