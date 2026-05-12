import { initSyntaxHighlighting } from "./syntax";

setTimeout(initSyntaxHighlighting, 0);

const initBranchSwitcher = () => {
	for (const ele of document.querySelectorAll<HTMLSelectElement>(
		"select.branch-switcher",
	)) {
		const base = ele.getAttribute("data-base-href");
		const mainBranch = ele.getAttribute("data-main-branch");
		const baseSuffix = ele.getAttribute("data-base-suffix") || "tree";
		const currentPath = ele.getAttribute("data-current-path") || "";
		if (!base) {
			continue;
		}
		const oldValue = ele.value;
		ele.onchange = () => {
			const newValue = ele.value;
			if (newValue === oldValue) {
				return;
			}

			const encodedValue = encodeURIComponent(newValue);

			// Handle tree/blob views: preserve current path
			if (baseSuffix === "tree" || baseSuffix === "blob") {
				const pathSuffix = currentPath ? `/${currentPath}` : "";
				document.location = `${base}/${baseSuffix}/${encodedValue}${pathSuffix}`;
			} else if (newValue === mainBranch) {
				document.location = `${base}`;
			} else {
				document.location = `${base}/${baseSuffix}/${encodedValue}`;
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

	const buttons = document.querySelectorAll<HTMLElement>(
		"header .show-navigation",
	);

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

const initCopyButtons = () => {
	for (const b of document.querySelectorAll<HTMLElement>(".copy-button")) {
		b.onclick = async () => {
			const value = b.getAttribute("copy-value");
			if (!value) {
				return;
			}
			await navigator.clipboard.writeText(value);
			const popup = document.createElement("DIV");
			popup.classList.add("button-popup");
			popup.innerText = "Copied";

			b.append(popup);
			popup.offsetTop;
			popup.classList.add("visible");

			const hide = (e: Event) => {
				e?.preventDefault();
				e?.stopPropagation();
				if (popup.classList.contains("hide")) {
					return;
				}
				popup.classList.add("hide");
				setTimeout(() => {
					popup.remove();
				}, 300);
			};
			setTimeout(hide, 1000);
			popup.onclick = hide;
		};
	}
};
setTimeout(initCopyButtons, 0);

const initProtocolSwitcher = () => {
	const urls = document.querySelectorAll<HTMLDivElement>(".clone-url");
	for (const url of urls) {
		const input = url.querySelector<HTMLInputElement>(`input[name="cloneUrl"]`);
		if (!input) {
			continue;
		}
		const sshUrl = input.getAttribute("data-ssh-url");
		const httpUrl = input.getAttribute("data-http-url");
		if (!sshUrl || !httpUrl) {
			continue;
		}

		const btns = url.querySelectorAll<HTMLButtonElement>("button.toggle-btn");
		for (const btn of btns) {
			btn.addEventListener("click", () => {
				if (btn.getAttribute("data-protocol") === "ssh") {
					input.value = sshUrl;
				} else {
					input.value = httpUrl;
				}
			});
		}
	}
};
setTimeout(initProtocolSwitcher, 0);

const initToggleButtons = () => {
	const groups = document.querySelectorAll<HTMLElement>(".btn-group");
	for (const group of groups) {
		const btns = group.querySelectorAll<HTMLElement>(".toggle-btn");
		for (const btn of btns) {
			btn.addEventListener("click", () => {
				for (const b of btns) {
					b.classList.remove("active");
				}
				btn.classList.add("active");
			});
		}
	}
};
setTimeout(initToggleButtons, 0);

type ModalOptions = {
	opener?: HTMLElement;
	label?: string;
};

const showModal = (content: Node, options: ModalOptions = {}) => {
	const backdrop = document.createElement("DIV");
	backdrop.classList.add("modal-backdrop");

	const panel = document.createElement("DIV");
	panel.classList.add("modal-panel");
	panel.setAttribute("role", "dialog");
	panel.setAttribute("aria-modal", "true");
	if (options.label) {
		panel.setAttribute("aria-label", options.label);
	}

	const closeButton = document.createElement("BUTTON");
	closeButton.setAttribute("type", "button");
	closeButton.classList.add("modal-close");
	closeButton.setAttribute("aria-label", "Close");
	closeButton.innerHTML = `<span class="icon i-x"></span>`;

	const close = () => {
		document.removeEventListener("keydown", onKeyDown);
		backdrop.remove();
		options.opener?.focus();
	};

	const onKeyDown = (event: KeyboardEvent) => {
		if (event.key === "Escape") {
			event.preventDefault();
			close();
		}
	};

	closeButton.onclick = close;
	backdrop.onclick = (event) => {
		if (event.target === backdrop) {
			close();
		}
	};

	panel.append(closeButton, content);
	backdrop.append(panel);
	document.body.append(backdrop);
	document.addEventListener("keydown", onKeyDown);

	for (const closeControl of panel.querySelectorAll<HTMLElement>(
		"[data-modal-close]",
	)) {
		closeControl.onclick = close;
	}

	const firstFocusable = panel.querySelector<HTMLElement>(
		'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
	);
	firstFocusable?.focus();

	return { close, element: backdrop };
};

const initProjectDelete = () => {
	const deleteButton = document.querySelector<HTMLButtonElement>(
		"#project-delete-button",
	);
	if (!deleteButton) {
		return;
	}

	const projectPath = deleteButton.getAttribute("data-project-path");
	if (!projectPath) {
		return;
	}

	deleteButton.onclick = (e) => {
		e.preventDefault();

		const confirmation = prompt(
			`To confirm deletion, type the project path exactly:\n${projectPath}`,
		);

		if (confirmation === null) {
			// User cancelled
			return;
		}

		if (confirmation.trim() !== projectPath) {
			alert("Confirmation text did not match. Deletion cancelled.");
			return;
		}

		// Submit the form
		const form = deleteButton.closest("form");
		if (form) {
			const confirmationInput = form.querySelector<HTMLInputElement>(
				'input[name="confirmation"]',
			);
			if (confirmationInput) {
				confirmationInput.value = confirmation;
			}
			form.submit();
		}
	};
};
setTimeout(initProjectDelete, 0);

const initForkDialogs = () => {
	for (const button of document.querySelectorAll<HTMLButtonElement>(
		".fork-button[data-modal-template]",
	)) {
		const templateId = button.getAttribute("data-modal-template");
		const template = templateId ? document.getElementById(templateId) : null;
		if (!(template instanceof HTMLTemplateElement)) {
			continue;
		}

		button.onclick = () => {
			const content = template.content.cloneNode(true);
			showModal(content, { opener: button, label: "Fork project" });
		};
	}
};
setTimeout(initForkDialogs, 0);
