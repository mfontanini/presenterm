presenterm
===

[![crates-badge]][crates-package] [![brew-badge]][brew-package] [![nix-badge]][nix-package] 
[![arch-badge]][arch-package] [![scoop-badge]][scoop-package]

[brew-badge]: https://img.shields.io/homebrew/v/presenterm
[brew-package]: https://formulae.brew.sh/formula/presenterm
[nix-badge]: https://img.shields.io/badge/Packaged_for-Nix-5277C3.svg?logo=nixos&labelColor=73C3D5
[nix-package]: https://search.nixos.org/packages?size=1&show=presenterm
[crates-badge]: https://img.shields.io/crates/v/presenterm
[crates-package]: https://crates.io/crates/presenterm
[arch-badge]: https://img.shields.io/archlinux/v/extra/x86_64/presenterm
[arch-package]: https://archlinux.org/packages/extra/x86_64/presenterm/
[scoop-badge]: https://img.shields.io/scoop/v/presenterm
[scoop-package]: https://scoop.sh/#/apps?q=presenterm&id=a462290f824b50f180afbaa6d8c7c1e6e0952e3a

_presenterm_ lets you create presentations in markdown format and run them from your terminal, with support for image 
and animated gifs, highly customizable themes, code highlighting, exporting presentations into PDF format, and plenty of 
other features. This is how the [demo presentation](/examples/demo.md) looks like when running in the [kitty 
terminal](https://sw.kovidgoyal.net/kitty/):

![](/docs/src/assets/demo.gif)

Check the rest of the example presentations in the [examples directory](/examples).

# Documentation

Visit the [documentation][docs-introduction] to get started.

# Features

* Define your presentation in a single markdown file.
* [Images and animated gifs][docs-images] on terminals like _kitty_, _iterm2_, and _wezterm_.
* [Customizeable themes][docs-themes] including colors, margins, layout (left/center aligned content), footer for every 
  slide, etc. Several [built-in themes][docs-builtin-themes] can give your presentation the look you want without 
  having to define your own.
* Code highlighting for a [wide list of programming languages][docs-code-highlight].
* [Font sizes][docs-font-sizes] for terminals that support them.
* [Selective/dynamic][docs-selective-highlight] code highlighting that only highlights portions of code at a time.
* [Column layouts][docs-layout].
* [mermaid graph rendering][docs-mermaid].
* [_LaTeX_ and _typst_ formula rendering][docs-latex].
* [Introduction slide][docs-intro-slide] that displays the presentation title and your name.
* [Slide titles][docs-slide-titles].
* [Snippet execution][docs-code-execute] for various programming languages.
* [Export presentations to PDF][docs-pdf-export].
* [Pause][docs-pauses] portions of your slides.
* [Custom key bindings][docs-key-bindings].
* [Automatically reload your presentation][docs-hot-reload] every time it changes for a fast development loop.
* [Define speaker notes][docs-speaker-notes] to aid you during presentations.

See the [introduction page][docs-introduction] to learn more.

<!-- links -->

[docs-introduction]: https://mfontanini.github.io/presenterm/
[docs-basics]: https://mfontanini.github.io/presenterm/features/introduction.html
[docs-intro-slide]: https://mfontanini.github.io/presenterm/features/introduction.html#introduction-slide
[docs-slide-titles]: https://mfontanini.github.io/presenterm/features/introduction.html#slide-titles
[docs-font-sizes]: https://mfontanini.github.io/presenterm/features/introduction.html#font-sizes
[docs-pauses]: https://mfontanini.github.io/presenterm/features/commands.html#pauses
[docs-images]: https://mfontanini.github.io/presenterm/features/images.html
[docs-themes]: https://mfontanini.github.io/presenterm/features/themes/introduction.html
[docs-builtin-themes]: https://mfontanini.github.io/presenterm/features/themes/introduction.html#built-in-themes
[docs-code-highlight]: https://mfontanini.github.io/presenterm/features/code/highlighting.html
[docs-code-execute]: https://mfontanini.github.io/presenterm/features/code/execution.html
[docs-selective-highlight]: https://mfontanini.github.io/presenterm/features/code/highlighting.html#selective-highlighting
[docs-layout]: https://mfontanini.github.io/presenterm/features/layout.html
[docs-mermaid]: https://mfontanini.github.io/presenterm/features/code/mermaid.html
[docs-latex]: https://mfontanini.github.io/presenterm/features/code/latex.html
[docs-pdf-export]: https://mfontanini.github.io/presenterm/features/pdf-export.html
[docs-key-bindings]: https://mfontanini.github.io/presenterm/configuration/settings.html#key-bindings
[docs-hot-reload]: https://mfontanini.github.io/presenterm/features/introduction.html#hot-reload
[docs-speaker-notes]: https://mfontanini.github.io/presenterm/features/speaker-notes.html
[bat]: https://github.com/sharkdp/bat
[syntect]: https://github.com/trishume/syntect


