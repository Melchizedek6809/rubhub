import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("hub-button")
export class HubButton extends LitElement {
	static styles = css`
.btn,
button {
    padding: var(--space-s) var(--space-m);
    background: var(--primary-color);
    color: var(--white);
    border: none;
    border-radius: var(--space-s);
    cursor: pointer;
    display: inline-block;
    line-height: 1.3em;
    font-size: 1rem;
    text-decoration: none;
    font-size: 1rem;
	line-height: 1.5em;
    transition: background-color 200ms ease-in-out;
}


.btn:hover,
.btn:focus,
button:hover,
button:focus {
	background-color: var(--primary-color-dark);
}
`;

	@property()
	href?: string;

	render() {
		if (this.href) {
			return html`<a class="btn" href="${this.href}"><slot /></a>`;
		} else {
			return html`<button class="btn" href="${this.href}"><slot /></button>`;
		}
	}
}
