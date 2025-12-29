import Prism from "prismjs";
import "prismjs/plugins/line-numbers/prism-line-numbers";
import "prismjs/plugins/match-braces/prism-match-braces";
import "prismjs/plugins/inline-color/prism-inline-color";

import "prismjs/components/prism-c";
import "prismjs/components/prism-ruby";
import "prismjs/components/prism-json";

// Newly included languages
import "prismjs/components/prism-abnf";
import "prismjs/components/prism-ada";
import "prismjs/components/prism-agda";
import "prismjs/components/prism-apl";
import "prismjs/components/prism-awk";
import "prismjs/components/prism-batch";
import "prismjs/components/prism-bison";
import "prismjs/components/prism-bnf";
import "prismjs/components/prism-clojure";
import "prismjs/components/prism-crystal";
import "prismjs/components/prism-csharp";
import "prismjs/components/prism-csv";
import "prismjs/components/prism-dart";
import "prismjs/components/prism-ebnf";
import "prismjs/components/prism-ejs";
import "prismjs/components/prism-elixir";
import "prismjs/components/prism-elm";
import "prismjs/components/prism-erlang";
import "prismjs/components/prism-fsharp";
import "prismjs/components/prism-gdscript";
import "prismjs/components/prism-glsl";
import "prismjs/components/prism-gradle";
import "prismjs/components/prism-handlebars";
import "prismjs/components/prism-haskell";
import "prismjs/components/prism-haxe";
import "prismjs/components/prism-hlsl";
import "prismjs/components/prism-idris";
import "prismjs/components/prism-ini";
import "prismjs/components/prism-json5";
import "prismjs/components/prism-jsonp";
import "prismjs/components/prism-jsx";
import "prismjs/components/prism-julia";
import "prismjs/components/prism-latex";
import "prismjs/components/prism-less";
import "prismjs/components/prism-lisp";
import "prismjs/components/prism-nasm";
import "prismjs/components/prism-nix";
import "prismjs/components/prism-ocaml";
import "prismjs/components/prism-pascal";
import "prismjs/components/prism-perl";
import "prismjs/components/prism-prolog";
import "prismjs/components/prism-reason";
import "prismjs/components/prism-rescript";
import "prismjs/components/prism-r";
import "prismjs/components/prism-sass";
import "prismjs/components/prism-scheme";
import "prismjs/components/prism-racket";
import "prismjs/components/prism-systemd";
import "prismjs/components/prism-tsx";
import "prismjs/components/prism-twig";

import "prismjs/components/prism-bash";
import "prismjs/components/prism-cpp";
import "prismjs/components/prism-css";
import "prismjs/components/prism-diff";
import "prismjs/components/prism-docker";
import "prismjs/components/prism-go";
import "prismjs/components/prism-graphql";
import "prismjs/components/prism-java";
import "prismjs/components/prism-kotlin";
import "prismjs/components/prism-lua";
import "prismjs/components/prism-makefile";
import "prismjs/components/prism-markdown";
import "prismjs/components/prism-markup-templating";
import "prismjs/components/prism-php";
import "prismjs/components/prism-python";
import "prismjs/components/prism-rust";
import "prismjs/components/prism-scss";
import "prismjs/components/prism-sql";
import "prismjs/components/prism-swift";
import "prismjs/components/prism-toml";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-yaml";
import "prismjs/components/prism-zig";

const extensionToLanguage: Record<string, string> = {
	// Shell
	bash: "bash",
	sh: "bash",
	zsh: "bash",
	bat: "batch",
	cmd: "batch",

	// C family
	c: "c",
	h: "c",
	cpp: "cpp",
	cxx: "cpp",
	cc: "cpp",
	hpp: "cpp",
	hxx: "cpp",
	hh: "cpp",
	cs: "csharp",

	// Web
	css: "css",
	less: "less",
	sass: "sass",
	scss: "scss",
	htm: "markup",
	html: "markup",
	svg: "markup",
	xml: "markup",
	js: "javascript",
	mjs: "javascript",
	jsx: "jsx",
	ts: "typescript",
	tsx: "tsx",
	json: "json",
	json5: "json5",
	jsonp: "jsonp",
	php: "php",
	ejs: "ejs",
	hbs: "handlebars",
	handlebars: "handlebars",
	twig: "twig",

	// Systems
	rs: "rust",
	go: "go",
	zig: "zig",
	swift: "swift",
	kt: "kotlin",
	java: "java",
	dart: "dart",
	cr: "crystal",

	// Scripting
	py: "python",
	rb: "ruby",
	lua: "lua",
	pl: "perl",
	pm: "perl",
	r: "r",
	jl: "julia",
	ex: "elixir",
	exs: "elixir",
	erl: "erlang",
	hrl: "erlang",

	// Functional
	hs: "haskell",
	lhs: "haskell",
	ml: "ocaml",
	mli: "ocaml",
	fs: "fsharp",
	fsi: "fsharp",
	fsx: "fsharp",
	elm: "elm",
	clj: "clojure",
	cljs: "clojure",
	cljc: "clojure",
	edn: "clojure",
	scm: "scheme",
	ss: "scheme",
	rkt: "racket",
	lisp: "lisp",
	lsp: "lisp",
	cl: "lisp",
	el: "lisp",
	re: "reason",
	rei: "reason",
	res: "rescript",
	resi: "rescript",
	idr: "idris",
	agda: "agda",

	// Config / Data
	yaml: "yaml",
	yml: "yaml",
	toml: "toml",
	ini: "ini",
	cfg: "ini",
	nix: "nix",
	dockerfile: "docker",
	makefile: "makefile",
	make: "makefile",
	gradle: "gradle",
	service: "systemd",

	// Other
	sql: "sql",
	md: "markdown",
	tex: "latex",
	latex: "latex",
	diff: "diff",
	patch: "diff",
	gql: "graphql",
	graphql: "graphql",
	csv: "csv",
	awk: "awk",
	pas: "pascal",
	p: "pascal",
	ada: "ada",
	adb: "ada",
	ads: "ada",
	hx: "haxe",
	gd: "gdscript",
	pro: "prolog",
	apl: "apl",

	// Low-level / Shaders
	asm: "nasm",
	nasm: "nasm",
	glsl: "glsl",
	vert: "glsl",
	frag: "glsl",
	hlsl: "hlsl",

	// Grammar
	abnf: "abnf",
	bnf: "bnf",
	ebnf: "ebnf",
	bison: "bison",
};

export const initSyntaxHighlighting = () => {
	const preBlock = document.querySelector<HTMLElement>("pre.file-content");
	if (!preBlock) return;

	const codeBlock = preBlock.querySelector<HTMLElement>("code");
	if (!codeBlock) return;

	const breadcrumb = document.querySelector(".path-breadcrumb");
	if (!breadcrumb) return;

	const filename = breadcrumb.textContent?.trim().split("/").pop() || "";
	const ext = filename.split(".").pop()?.toLowerCase() || "";
	const language = extensionToLanguage[ext] || extensionToLanguage[filename];

	// Add line-numbers and rainbow-braces classes to pre element
	preBlock.classList.add("line-numbers", "rainbow-braces", "match-braces");

	if (language) {
		codeBlock.classList.add(`language-${language}`);
	}

	Prism.highlightElement(codeBlock);
};
