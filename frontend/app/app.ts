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
	browseLink.href = user ? `/${user.username}` : "/login";
}

const initBranchSwitcher = () => {
	for (const ele of document.querySelectorAll<HTMLSelectElement>(
		"select.branch-switcher",
	)) {
		const base = ele.getAttribute("data-base-href");
		const mainBranch = ele.getAttribute("data-main-branch");
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
				document.location = `${base}/tree/${newValue}`;
			}
		};
	}
};
setTimeout(initBranchSwitcher, 0);
