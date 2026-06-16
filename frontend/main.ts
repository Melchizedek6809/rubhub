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
