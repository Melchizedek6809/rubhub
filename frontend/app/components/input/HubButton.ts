import {LitElement, css, html} from 'lit';
import {customElement, property} from 'lit/decorators.js';

@customElement('hub-button')
export class HubButton extends LitElement {

  static styles = css`
.btn,
button {
    padding: var(--space-s) var(--space-m);
    background: var(--primary-color);
    color: var(--white);
    border: solid 1px var(--primary-color-bright);
    border-bottom-color: var(--primary-color-dark);
    border-right-color: var(--primary-color-dark);
    border-radius: var(--space-s);
    cursor: pointer;
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