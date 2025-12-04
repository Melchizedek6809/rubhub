import { html, LitElement } from "lit";
import { customElement, state } from "lit/decorators.js";

@customElement("user-nav")
export class UserNav extends LitElement {
	@state()
	private user: { id: string; username: string } | null = null;

	connectedCallback() {
		super.connectedCallback();
		this.user = this.readUserCookie();
	}

	private readUserCookie(): { id: string; username: string } | null {
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

	createRenderRoot() {
		// Render light DOM so styling can be inherited from layout.
		return this;
	}

	render() {
		if (!this.user) return null;

		return html`
      <nav aria-label="User navigation">
        <ul style="list-style:none; padding:0; margin:0; display:flex; gap:8px;">
          <li><a href="/${this.user.username}/projects">Projects</a></li>
        </ul>
      </nav>
    `;
	}
}
