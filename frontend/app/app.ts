type SessionUser = {
	id: string;
	username: string;
};

function readSessionUser(): SessionUser | null {
	const cookie = document.cookie
		.split("; ")
		.find((row) => row.startsWith("session_user="));

	if (!cookie) return null;

	try {
		const value = decodeURIComponent(cookie.split("=")[1]);
		const parsed = JSON.parse(value);
		if (parsed?.id && parsed?.username) {
			return { id: parsed.id, username: parsed.username };
		}
	} catch (_) {
		return null;
	}

	return null;
}

const browseLink = document.querySelector<HTMLAnchorElement>("#browse-link");
if (browseLink) {
	const user = readSessionUser();
	browseLink.href = user ? `/~${user.username}` : "/login";
}

const initBranchSwitcher = () => {
	for (const ele of document.querySelectorAll<HTMLSelectElement>(
		"select.branch-switcher",
	)) {
		const base = ele.getAttribute("data-base-href");
		const mainBranch = ele.getAttribute("data-main-branch");
		const baseSuffix = ele.getAttribute("data-base-suffix") || "tree";
		if (!base) {
			continue;
		}
		const oldValue = ele.value;
		ele.onchange = () => {
			const newValue = ele.value;
			if (newValue === oldValue) {
				return;
			}

			if (newValue === mainBranch) {
				document.location = `${base}`;
			} else {
				document.location = `${base}/${baseSuffix}/${newValue}`;
			}
		};
	}
};
setTimeout(initBranchSwitcher, 0);

const initSidebar = () => {
	const sidebar = document.querySelector("#sidebar");
	if (!sidebar) {
		return;
	}

	const main = document.querySelector("main");
	if (!main) {
		return;
	}

	const buttons = document.querySelectorAll<HTMLElement>("header .show-navigation");

	const toggleSidebar = () => {
		sidebar.classList.toggle("visible");
		for (const b of buttons) {
			if (sidebar.classList.contains("visible")) {
				b.classList.add("active");
			} else {
				b.classList.remove("active");
			}
		}
	};
	main.onclick = () => {
		if (sidebar.classList.contains("visible")) {
			toggleSidebar();
		}
	};

	for (const button of buttons) {
		button.onclick = () => {
			toggleSidebar();
		};
	}
};
setTimeout(initSidebar, 0);
