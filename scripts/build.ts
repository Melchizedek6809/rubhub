import { $ } from "bun";

await $`rm -rf ./dist/`;

await Promise.all([
	$`NODE_ENV=production bun build --minify ./frontend/app/app.html --outdir ./dist/ --public-path /dist/`,
]);
