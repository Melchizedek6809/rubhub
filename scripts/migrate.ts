#!/usr/bin/env bun

import { readdirSync } from "node:fs";
import { join } from "node:path";
import { SQL } from "bun";

const MIGRATIONS_DIR = "./migrations";

const dbUrl = process.env.DATABASE_URL || "postgresql://postgres:postgres@localhost:5432/rubhub";
const pg = new SQL(dbUrl);

async function ensureMigrationsTable() {
	await pg`
	CREATE TABLE IF NOT EXISTS migrations (
		name TEXT PRIMARY KEY,
		applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
	);
	`;
}

async function getAppliedMigrations(): Promise<Set<string>> {
	const res = await pg`SELECT name FROM migrations;`;
	const set = new Set<string>();
	for (const row of res) {
		set.add(row.name);
	}
	return set;
}

async function applyMigration(name: string, sqlFilePath: string) {
	console.log(`→ Applying ${name} ...`);
	try {
		await pg.begin(async (tx) => {
			await tx.file(sqlFilePath);
			await tx`INSERT INTO migrations (name) VALUES (${name})`;
		});
		console.log(`✓ Done ${name}`);
	} catch (err) {
		console.error(`✗ Failed ${name}`, err);
		process.exit(1);
	}
}

async function main() {
	await ensureMigrationsTable();
	const applied = await getAppliedMigrations();
	const files = readdirSync(MIGRATIONS_DIR)
		.filter((f) => f.endsWith(".sql"))
		.sort();

	for (const file of files) {
		if (applied.has(file)) {
			console.log(`→ Skipping ${file} (already applied)`);
		} else {
			await applyMigration(file, join(MIGRATIONS_DIR, file));
		}
	}
	console.log("All migrations are up to date.");
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
