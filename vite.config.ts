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
			// Move app.html from dist/frontend/app/ to dist/
			const src = "dist/frontend/app/app.html";
			const dest = "dist/app.html";
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
		emptyOutDir: true,
		reportCompressedSize: process.env.NODE_ENV === "production",
		minify: process.env.NODE_ENV === "production",
		modulePreload: {
			polyfill: false,
		},
		rollupOptions: {
			input: {
				app: resolve(__dirname, "frontend/app/app.html"),
			},
			output: {
				entryFileNames: "app-[hash].js",
				chunkFileNames: "app-[hash].js",
				assetFileNames: (assetInfo) => {
					if (assetInfo.name?.endsWith(".css")) {
						return "app-[hash].css";
					}
					return "assets/[name]-[hash][extname]";
				},
			},
		},
	},
});
