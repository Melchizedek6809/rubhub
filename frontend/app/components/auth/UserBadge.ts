import {LitElement, html} from 'lit';
import {customElement, state} from 'lit/decorators.js';

@customElement('user-badge')
export class UserBadge extends LitElement {
  @state()
  private user: {id: string; username: string} | null = null;

  connectedCallback() {
    super.connectedCallback();
    this.user = this.readUserCookie();
  }

  private readUserCookie(): {id: string; username: string} | null {
    const cookie = document.cookie
      .split('; ')
      .find((row) => row.startsWith('session_user='));

    if (!cookie) return null;

    try {
      const value = decodeURIComponent(cookie.split('=')[1]);
      const parsed = JSON.parse(value);
      if (parsed?.id && parsed?.username) {
        return {id: parsed.id, username: parsed.username};
      }
    } catch (_) {
      return null;
    }

    return null;
  }

  private handleLogout() {
    window.location.href = '/logout';
  }

  render() {
    if (this.user) {
      return html`
        <span>${this.user.username}</span>
        <hub-button @click=${this.handleLogout}>Logout</hub-button>
      `;
    }

    return html`<hub-button href="/login">Login</hub-button>`;
  }
}
