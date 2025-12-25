import EasyMDE from "easymde";
import "easymde/dist/easymde.min.css";

export const initMarkdownEditors = () => {
	for (const textarea of document.querySelectorAll<HTMLTextAreaElement>(
		"textarea.markdown-editor",
	)) {
		const isRequired = textarea.hasAttribute("required");
		textarea.removeAttribute("required");

		const editor = new EasyMDE({
			element: textarea,
			spellChecker: false,
			status: false,
			toolbar: [
				"bold",
				"italic",
				"heading",
				"|",
				"quote",
				"unordered-list",
				"ordered-list",
				"|",
				"link",
				"image",
				"code",
				"|",
				"preview",
				"guide",
			],
			previewClass: ["editor-preview", "markdown"],
		});

		if (isRequired) {
			const form = textarea.closest("form");
			form?.addEventListener("submit", (e) => {
				if (!editor.value().trim()) {
					e.preventDefault();
					editor.codemirror.focus();
				}
			});
		}
	}
};
