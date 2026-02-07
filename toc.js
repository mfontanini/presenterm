// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Docs</li><li class="chapter-item expanded "><a href="install.html"><strong aria-hidden="true">1.</strong> Install</a></li><li class="chapter-item expanded "><a href="features/introduction.html"><strong aria-hidden="true">2.</strong> Features</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="features/images.html"><strong aria-hidden="true">2.1.</strong> Images</a></li><li class="chapter-item expanded "><a href="features/commands.html"><strong aria-hidden="true">2.2.</strong> Commands</a></li><li class="chapter-item expanded "><a href="features/layout.html"><strong aria-hidden="true">2.3.</strong> Layout</a></li><li class="chapter-item expanded "><a href="features/code/highlighting.html"><strong aria-hidden="true">2.4.</strong> Code</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="features/code/execution.html"><strong aria-hidden="true">2.4.1.</strong> Execution</a></li><li class="chapter-item expanded "><a href="features/code/mermaid.html"><strong aria-hidden="true">2.4.2.</strong> Mermaid diagrams</a></li><li class="chapter-item expanded "><a href="features/code/latex.html"><strong aria-hidden="true">2.4.3.</strong> LaTeX and typst</a></li><li class="chapter-item expanded "><a href="features/code/d2.html"><strong aria-hidden="true">2.4.4.</strong> D2</a></li></ol></li><li class="chapter-item expanded "><a href="features/themes/introduction.html"><strong aria-hidden="true">2.5.</strong> Themes</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="features/themes/definition.html"><strong aria-hidden="true">2.5.1.</strong> Definition</a></li></ol></li><li class="chapter-item expanded "><a href="features/exports.html"><strong aria-hidden="true">2.6.</strong> Exports</a></li><li class="chapter-item expanded "><a href="features/slide-transitions.html"><strong aria-hidden="true">2.7.</strong> Slide transitions</a></li><li class="chapter-item expanded "><a href="features/speaker-notes.html"><strong aria-hidden="true">2.8.</strong> Speaker notes</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/introduction.html"><strong aria-hidden="true">3.</strong> Configuration</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/options.html"><strong aria-hidden="true">3.1.</strong> Options</a></li><li class="chapter-item expanded "><a href="configuration/settings.html"><strong aria-hidden="true">3.2.</strong> Settings</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Internals</li><li class="chapter-item expanded "><a href="internals/parse.html"><strong aria-hidden="true">4.</strong> Parse</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><a href="acknowledgements.html">Acknowledgements</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
