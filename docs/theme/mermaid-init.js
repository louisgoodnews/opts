// Renders ```mermaid code blocks without the mdbook-mermaid preprocessor.
//
// The preprocessor is the usual route (book.toml: [preprocessor.mermaid]); it is not installed
// in the build environment yet, and a missing preprocessor binary fails the whole build. This
// script does the same job in the browser: it turns each `code.language-mermaid` block into a
// `pre.mermaid` element and loads Mermaid from a pinned CDN version.
(function () {
  "use strict";

  var MERMAID_URL = "https://cdn.jsdelivr.net/npm/mermaid@11.4.1/dist/mermaid.min.js";

  var blocks = document.querySelectorAll("code.language-mermaid");
  if (blocks.length === 0) {
    return;
  }

  blocks.forEach(function (code) {
    var pre = code.parentElement;
    var diagram = document.createElement("pre");
    diagram.className = "mermaid";
    diagram.textContent = code.textContent;
    pre.replaceWith(diagram);
  });

  var dark = ["navy", "coal", "ayu"].some(function (name) {
    return document.documentElement.classList.contains(name);
  });

  var script = document.createElement("script");
  script.src = MERMAID_URL;
  script.async = true;
  script.onload = function () {
    window.mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      theme: dark ? "dark" : "default"
    });
    window.mermaid.run({ querySelector: "pre.mermaid" });
  };
  script.onerror = function () {
    // Without the CDN the diagrams stay readable as their source text.
    document.querySelectorAll("pre.mermaid").forEach(function (el) {
      el.classList.add("mermaid-unavailable");
    });
  };
  document.head.appendChild(script);
})();
