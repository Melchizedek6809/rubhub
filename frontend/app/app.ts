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
	browseLink.href = user ? `/${user.username}/projects` : "/login";
}
