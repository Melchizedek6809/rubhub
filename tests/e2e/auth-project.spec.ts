import { expect, type Page, test } from "@playwright/test";

type TestUser = {
	username: string;
	password: string;
	email: string;
};

type TestProject = {
	name: string;
	slug: string;
	description: string;
};

function uniqueName(prefix: string) {
	return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function createTestUser(): TestUser {
	const username = uniqueName("e2euser");

	return {
		username,
		password: "playwright-password-12345",
		email: `${username}@example.com`,
	};
}

function createTestProject(prefix = "E2E Project"): TestProject {
	const name = uniqueName(prefix);

	return {
		name,
		slug: name.toLowerCase().replaceAll(/[^a-z0-9]+/g, "-"),
		description: "Description updated by the Playwright smoke test.",
	};
}

async function registerUser(page: Page, user: TestUser) {
	await page.goto("/registration");
	await page.locator('input[name="username"]').fill(user.username);
	await page.locator('input[name="email"]').fill(user.email);
	await page.locator('input[name="password"]').fill(user.password);
	await page.locator('form[action="/registration"] button[type="submit"]').click();

	await expect(page).toHaveURL(new RegExp(`/~${user.username}$`));
}

async function expectLoggedInAs(page: Page, user: TestUser) {
	await expect(page.locator(".username-link").first()).toHaveText(user.username);
	await expect(page.locator('form[action="/logout"] button[type="submit"]').first()).toBeVisible();
}

async function logout(page: Page) {
	await page.locator('form[action="/logout"] button[type="submit"]').first().click();
	await expect(page.locator('a[href="/login"]').first()).toBeVisible();
}

async function login(page: Page, user: TestUser) {
	await page.goto("/login");
	await page.locator('input[name="username"]').fill(user.username);
	await page.locator('input[name="password"]').fill(user.password);
	await page.locator('form[action="/login"] button[type="submit"]').click();

	await expect(page).toHaveURL(new RegExp(`/~${user.username}$`));
}

async function createProject(page: Page, user: TestUser, project: TestProject) {
	await page.goto("/projects/new");
	await page.locator('input[name="name"]').fill(project.name);
	await page.locator('form[action="/projects/new"] button[type="submit"]').click();

	await expect(page).toHaveURL(new RegExp(`/~${user.username}/${project.slug}$`));
	await expect(
		page.getByRole("heading", {
			name: new RegExp(`${user.username}\\s*/\\s*${project.name}`),
		}),
	).toBeVisible();
}

async function updateProjectDescription(
	page: Page,
	user: TestUser,
	project: TestProject,
) {
	await page.goto(`/~${user.username}/${project.slug}/settings`);
	await page.locator('textarea[name="description"]').fill(project.description);
	await page.locator('form[action$="/settings"] button[type="submit"]').first().click();

	await expect(page).toHaveURL(
		new RegExp(`/~${user.username}/${project.slug}/settings$`),
	);
	await expect(page.locator('textarea[name="description"]')).toHaveValue(
		project.description,
	);
}

async function expectProjectOnUserProfile(
	page: Page,
	user: TestUser,
	project: TestProject,
) {
	await page.goto(`/~${user.username}`);
	await expectProjectInUserProjectList(page, project);
}

async function expectProjectInUserProjectList(page: Page, project: TestProject) {
	const projectCard = userProjectCard(page, project);

	await expect(projectCard).toContainText(project.name);
	await expect(projectCard).toContainText(project.description);
}

async function expectProjectNotInUserProjectList(
	page: Page,
	project: TestProject,
) {
	await expect(userProjectCard(page, project)).toHaveCount(0);
}

async function expectProjectOnProjectsPage(
	page: Page,
	user: TestUser,
	project: TestProject,
) {
	await page.goto("/projects");
	await expectProjectInGlobalProjectList(page, user, project);
}

async function expectProjectInGlobalProjectList(
	page: Page,
	user: TestUser,
	project: TestProject,
) {
	const projectCard = globalProjectCard(page, user, project);

	await expect(projectCard).toContainText(`${user.username}/${project.name}`);
	await expect(projectCard).toContainText(project.description);
}

async function expectProjectNotInGlobalProjectList(
	page: Page,
	user: TestUser,
	project: TestProject,
) {
	await expect(globalProjectCard(page, user, project)).toHaveCount(0);
}

function userProjectCard(page: Page, project: TestProject) {
	return page.locator(".project-card").filter({ hasText: project.name });
}

function globalProjectCard(page: Page, user: TestUser, project: TestProject) {
	return page
		.locator(".project-card")
		.filter({ hasText: `${user.username}/${project.name}` });
}

async function deleteProject(page: Page, user: TestUser, project: TestProject) {
	page.once("dialog", async (dialog) => {
		await dialog.accept(`${user.username}/${project.slug}`);
	});

	await page.goto(`/~${user.username}/${project.slug}/settings`);
	await page.locator("#project-delete-button").click();
	await expect(page).toHaveURL(new RegExp(`/~${user.username}$`));
}

async function deleteUser(page: Page, user: TestUser) {
	await page.goto("/settings");
	await page.locator('input[name="confirmation"]').fill(user.username);
	await page.locator('form[action^="/settings/delete/"] button[type="submit"]').click();

	await expect(page).toHaveURL("/");
	await expect(page.locator('a[href="/login"]').first()).toBeVisible();
}

test("user can register, log in, create a project, and publish a description", async ({
	page,
}) => {
	const user = createTestUser();
	const firstProject = createTestProject("E2E First Project");
	const secondProject = createTestProject("E2E Second Project");

	await registerUser(page, user);
	await expectLoggedInAs(page, user);
	await logout(page);
	await login(page, user);
	await expectLoggedInAs(page, user);
	await createProject(page, user, firstProject);
	await updateProjectDescription(page, user, firstProject);
	await createProject(page, user, secondProject);
	await updateProjectDescription(page, user, secondProject);

	await page.goto(`/~${user.username}`);
	await expectProjectInUserProjectList(page, firstProject);
	await expectProjectInUserProjectList(page, secondProject);

	await page.goto("/projects");
	await expectProjectInGlobalProjectList(page, user, firstProject);
	await expectProjectInGlobalProjectList(page, user, secondProject);

	await deleteProject(page, user, secondProject);
	await expectProjectOnUserProfile(page, user, firstProject);
	await expectProjectNotInUserProjectList(page, secondProject);

	await page.goto("/projects");
	await expectProjectInGlobalProjectList(page, user, firstProject);
	await expectProjectNotInGlobalProjectList(page, user, secondProject);

	await deleteUser(page, user);
	await page.goto("/projects");
	await expectProjectNotInGlobalProjectList(page, user, firstProject);
});
