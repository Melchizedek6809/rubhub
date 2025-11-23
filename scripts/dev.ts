import { $ } from "bun";

await $`rm -rf ./dist/`;

await Promise.all([
	$`bun build --watch ./frontend/app/app.html --outdir ./dist/ --public-path /dist/`,
]);
