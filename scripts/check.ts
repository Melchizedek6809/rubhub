import { $ } from "bun";

await Promise.all([
	$`bunx tsc --project ./frontend/app/tsconfig.json`,
]);
