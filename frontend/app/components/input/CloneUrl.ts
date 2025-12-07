import { css, html, LitElement } from "lit";
import { customElement, property, query, state } from "lit/decorators.js";

@customElement("rubhub-clone-url")
export class RubhubCloneUrl extends LitElement {
	static styles = css`
		:host {
			display: block;
			position: relative;
		}

		#input-row {
			display: flex;
			align-items: stretch;
		}

		label {
			font-weight: 600;
		}

		input {
			padding: var(--space-m);
			background: var(--background-color);
			color: var(--text-color);
			border: solid 1px var(--primary-color);
			border-radius: var(--space-s) 0 0 var(--space-s);
			border-right: none;
			width: 100%;
			font-size: 1rem;
			line-height: 1.5em;
		}

		.copy-btn {
			display: inline-flex;
			align-items: center;
			gap: var(--space-xs);
			white-space: nowrap;
			padding: var(--space-m);
			margin: 0;
			background: var(--primary-color);
			color: var(--white);
			border: 1px solid var(--primary-color);
			border-left: none;
			border-radius: 0 var(--space-s) var(--space-s) 0;
			cursor: pointer;
			font-size: 1rem;
			line-height: 1.5em;
		}

		#status {
			min-height: 1.2rem;
			background: var(--success-color-dark);
			color: var(--white);
			border-radius: var(--space-s);
			padding: var(--space-s) var(--space-m);

			position: absolute;
			right: 0;
			top: 100%;
			margin-top:8px;
			transform: scaleY(0);
			transform-origin: 50% 0;
			opacity: 0;
			transition: opacity 400ms ease, transform 500ms cubic-bezier(0.68, -0.55, 0.265, 1.55);
		}

		#status.error {
			background: var(--error-color);
		}

		#status.visible {
			transform: scaleY(1);
			opacity: 1;
		}
	`;

	@property({ type: String })
	label = "";

	@property({ type: String })
	value = "";

	@state()
	private status: "idle" | "copied" | "error" = "idle";

	@state()
	private statusMessage: string = "";

	@query("input")
	private input?: HTMLInputElement;

	private statusTimeout: number | undefined;

	disconnectedCallback() {
		super.disconnectedCallback();
		if (this.statusTimeout) {
			window.clearTimeout(this.statusTimeout);
		}
	}

	private async copy() {
		if (!this.value) return;

		try {
			await navigator.clipboard.writeText(this.value);
			this.setStatus("copied");
			return;
		} catch (_) {
			// Fallback to selection-based copy for older browsers or blocked clipboard access
			if (this.input) {
				this.input.focus();
				this.input.select();
				const ok = document.execCommand("copy");
				this.setStatus(ok ? "copied" : "error");
				return;
			}
		}

		this.setStatus("error");
	}

	private getStatusMessage(status: "idle" | "copied" | "error") {
		switch (status) {
			case "idle":
				return "";
			case "copied":
				return "Copied!";
			case "error":
				return "Could not copy. Please copy manually.";
		}
	}

	private setStatus(next: "idle" | "copied" | "error") {
		this.status = next;
		if (next !== "idle") {
			this.statusMessage = this.getStatusMessage(this.status);
		}

		if (this.statusTimeout) {
			window.clearTimeout(this.statusTimeout);
		}

		if (next !== "idle") {
			this.statusTimeout = window.setTimeout(() => {
				this.status = "idle";
			}, 2000);
		}
	}

	render() {
		const showStatus = this.status !== "idle";

		return html`
			${this.label && html`<label>${this.label}</label>`}
			<div id="input-row">
				<input name="cloneUrl" type="text" readonly .value=${this.value} aria-label=${this.label} />
				<button class="copy-btn" type="button" @click=${this.copy}>Copy</button>
			</div>
			<div id="status" class="${showStatus ? "visible" : ""} ${this.status === "copied" ? "success" : ""} ${this.status === "error" ? "error" : ""}" role="status" aria-live="polite">
				${this.statusMessage}
			</div>
		`;
	}
}
