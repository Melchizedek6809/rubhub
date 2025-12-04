import { css, html, LitElement } from "lit";
import { customElement, property, state } from "lit/decorators.js";

@customElement("rubhub-slug-input")
export class RubhubSlugInput extends LitElement {
	static formAssociated = true;

	private internals: ElementInternals;

	private inputId = `slug-${Math.random().toString(36).slice(2, 8)}`;

	@property({ type: String })
	label = "";

	@property({ type: String })
	name = "";

	@property({ type: String })
	value = "";

	@property({ type: String })
	placeholder = "";

	@property({ type: String })
	autocomplete = "off";

	@property({ type: Boolean })
	disabled = false;

	@state()
	private status: "idle" | "valid" | "invalid" = "idle";

	@state()
	private errorMessage = "";

	private debounceId: number | undefined;

	static styles = css`
		:host {
			display: block;
			position: relative;
		}

		.field {
			display: flex;
			flex-direction: column;
			gap: var(--space-xs);
		}

		label {
			font-weight: 600;
		}

		.input-wrap {
			position: relative;
			display: inline-flex;
			align-items: center;
			width: 100%;
		}

		input {
			width: 100%;
			padding: var(--space-m);
			padding-right: calc(2.5rem + --space-m);
			background: var(--background-color);
			color: var(--text-color);
			border: solid 1px var(--primary-color);
			border-radius: var(--space-s);
			margin-bottom: var(--space-m);
		}

		.icon {
			position: absolute;
			right: 0.75rem;
			top: 0.35rem;
			display: inline-flex;
			align-items: center;
			justify-content: center;
			width: 1.25rem;
			height: 1.25rem;
			font-size: 1.75rem;
			opacity: 0;
			transform: scale(0);
			transition: opacity 200ms ease, transform 300ms cubic-bezier(0.68, -0.55, 0.265, 1.55);
		}

		.icon.visible {
			opacity: 1;
			transform: scale(1);
		}

		.icon#success {
			color: var(--success-color);
		}

		.icon#error {
			color: var(--error-color);
		}

		#message {
			min-height: 1.2rem;
			background: var(--error-color);
			color: var(--white);
			border-radius: var(--space-s);
			padding: var(--space-s) var(--space-m);

			position: absolute;
			left: 0;
			top: 100%;
			transform: scaleY(0);
			transform-origin: 50% 0;
			opacity: 0;
			transition: opacity 400ms ease, transform 500ms cubic-bezier(0.68, -0.55, 0.265, 1.55);
		}

		#message.visible {
			transform: scaleY(1);
			opacity: 1;
		}
	`;

	constructor() {
		super();
		this.internals = this.attachInternals();
	}

	disconnectedCallback() {
		super.disconnectedCallback();
		if (this.debounceId) {
			window.clearTimeout(this.debounceId);
		}
	}

	updated(changed: Map<string, unknown>) {
		if (changed.has("value")) {
			this.syncFormValue();
		}
	}

	private syncFormValue() {
		this.internals.setFormValue(this.value);
	}

	private handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		this.value = target.value;
		this.queueValidation();
		this.dispatchEvent(new Event("input", { bubbles: true, composed: true }));
	}

	private handleBlur() {
		if (this.debounceId) {
			window.clearTimeout(this.debounceId);
		}
		this.runValidation();
	}

	private queueValidation() {
		if (this.debounceId) {
			window.clearTimeout(this.debounceId);
		}
		this.debounceId = window.setTimeout(() => {
			this.runValidation();
		}, 200);
	}

	private runValidation() {
		const message = this.validateSlug(this.value);
		if (message && message !== this.errorMessage) {
			this.errorMessage = message;
		}
		this.status = message ? "invalid" : "valid";

		const input = this.renderRoot.querySelector("input") ?? undefined;
		if (message) {
			this.internals.setValidity({ customError: true }, message, input);
		} else {
			this.internals.setValidity({});
		}

		this.syncFormValue();
		this.requestUpdate();
	}

	private validateSlug(value: string): string | null {
		if (value.length < 3) {
			return "Value must be at least 3 characters.";
		}

		if (value.startsWith(".")) {
			return "Value cannot start with a period.";
		}

		const allowed = /^[a-z0-9._-]+$/;
		if (!allowed.test(value)) {
			return "Only lowercase letters, numbers, dashes, underscores, and periods are allowed.";
		}

		return null;
	}

	render() {
		const showIcon = this.status !== "idle";
		const isValid = this.status === "valid";
		const isInvalid = this.status === "invalid";
		const showMessage = isInvalid && this.errorMessage.length > 0;

		return html`
			<div class="field">
				<label for="${this.inputId}">${this.label || ""}</label>
				<div class="input-wrap">
					<input
						id=${this.inputId}
						name=${this.name}
						.value=${this.value}
						.placeholder=${this.placeholder}
						?disabled=${this.disabled}
						autocomplete=${this.autocomplete || "off"}
						autocorrect="off"
						spellcheck="false"
						inputmode="text"
						@input=${this.handleInput}
						@blur=${this.handleBlur}
					/>
					<span id="success" class="icon ${showIcon && isValid ? "visible" : ""}" aria-hidden="true">✔</span>
					<span id="error" class="icon ${showIcon && !isValid ? "visible" : ""}" aria-hidden="true">✕</span>
				</div>
				<div id="message" class="${showMessage ? "visible" : ""}" role="alert">
					${this.errorMessage}
				</div>
			</div>
		`;
	}
}
