// Switching language keeps you on the same page, scrolled to the same place
// and every other link keeps you in the language you picked
// e.g. on it-index.html, the link to blog/first-post.html goes to blog/it-first-post.html instead
document.addEventListener("DOMContentLoaded", () => {
	const page = location.pathname.split("/").pop() || "index.html";
	const language = /^([a-z]{2})-/.exec(page)?.[1];
	const pageWithoutLanguage = page.replace(/^[a-z]{2}-/, "");

	// The language links: go to this same page, in the other language
	document.querySelectorAll(".signpost a").forEach((link) => {
		const linkLanguage = /([a-z]{2})-[^/]*$/.exec(link.getAttribute("href") || "")?.[1];
		link.setAttribute("href", `${linkLanguage}-${pageWithoutLanguage}`);
		link.addEventListener("click", () => sessionStorage.setItem("scroll", String(window.scrollY)));
	});

	// Scroll back to where you were before switching
	const scroll = sessionStorage.getItem("scroll");
	if (scroll) {
		sessionStorage.removeItem("scroll");
		window.scrollTo(0, Number(scroll));
	}

	// Every other link: stay in the same language
	if (!language) return;
	document.documentElement.lang = language;
	document.querySelectorAll("a[href]").forEach((link) => {
		const href = link.getAttribute("href") || "";
		const parts = href.split("/");
		const file = parts.pop() || "";
		if (href.includes(":") || href.startsWith("#") || /^[a-z]{2}-/.test(file)) return; // other websites, or already has a language
		link.setAttribute("href", [...parts, `${language}-${file}`].join("/"));
	});
});
