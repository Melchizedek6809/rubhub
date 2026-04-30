import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
	testDir: "./tests/e2e",
	timeout: 30_000,
	expect: {
		timeout: 5_000,
	},
	fullyParallel: false,
	reporter: "list",
	webServer: {
		command:
			"DIR_ROOT=/tmp/rubhub-e2e HTTP_BIND_ADDRESS=127.0.0.1:3100 SSH_BIND_ADDRESS=127.0.0.1:0 cargo run --quiet",
		url: "http://127.0.0.1:3100",
		timeout: 30_000,
		reuseExistingServer: !process.env.CI,
	},
	use: {
		baseURL: "http://127.0.0.1:3100",
		trace: "retain-on-failure",
		screenshot: "only-on-failure",
	},
	projects: [
		{
			name: "chromium",
			use: { ...devices["Desktop Chrome"] },
		},
	],
});
