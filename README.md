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
[arch-badge]: https://img.shields.io/aur/version/presenterm-bin
[arch-package]: https://aur.archlinux.org/packages/presenterm-bin
[scoop-badge]: https://img.shields.io/scoop/v/presenterm
[scoop-package]: https://scoop.sh/#/apps?q=presenterm&id=a462290f824b50f180afbaa6d8c7c1e6e0952e3a

_presenterm_ lets you create presentations in markdown format and run them from your terminal, with support for image 
and animated gif support, highly customizable themes, code highlighting, exporting presentations into PDF format, and 
plenty of other features. This is how the [demo presentation](examples/demo.md) looks like:

![](/docs/src/assets/demo.gif)

# Documentation

Visit the [documentation][guide-introduction] to get started.

# Features

* Define your presentation in a single markdown file.
* [Images and animated gifs][guide-images] on terminals like _kitty_, _iterm2_, and _wezterm_.
* [Customized themes][guide-themes] including colors, margins, layout (left/center aligned content), footer for every 
  slide, etc.
* Code highlighting for a [wide list of programming languages][guide-code-highlight].
* [Selective/dynamic][guide-selective-highlight] code highlighting that only highlights portions of code at a time.
* [Column layouts][guide-layout].
* [_LaTeX_ and _typst_ formula rendering][guide-latex].
* [Introduction slide][guide-intro-slide] that displays the presentation title and your name.
* [Slide titles][guide-slide-titles].
* [Shell code execution][guide-code-execute].
* [Export presentations to PDF][guide-pdf-export].
* [Pause][guide-pauses] portions of your slides.
* [Custom key bindings][guide-key-bindings].
* [Automatically reload your presentation][guide-hot-reload] every time it changes for a fast development loop.

See the [introduction page][guide-introduction] to learn more.

<!-- links -->

[guide-introduction]: https://mfontanini.github.io/presenterm/
[guide-installation]: https://mfontanini.github.io/presenterm/guides/installation.html
[guide-basics]: https://mfontanini.github.io/presenterm/guides/basics.html
[guide-intro-slide]: https://mfontanini.github.io/presenterm/guides/basics.html#introduction-slide
[guide-slide-titles]: https://mfontanini.github.io/presenterm/guides/basics.html#slide-titles
[guide-pauses]: https://mfontanini.github.io/presenterm/guides/basics.html#pauses
[guide-images]: https://mfontanini.github.io/presenterm/guides/basics.html#images
[guide-themes]: https://mfontanini.github.io/presenterm/guides/themes.html
[guide-builtin-themes]: https://mfontanini.github.io/presenterm/guides/themes.html#built-in-themes
[guide-code-highlight]: https://mfontanini.github.io/presenterm/guides/code-highlight.html
[guide-code-execute]: https://mfontanini.github.io/presenterm/guides/code-highlight.html#executing-code
[guide-selective-highlight]: https://mfontanini.github.io/presenterm/guides/code-highlight.html#selective-highlighting
[guide-layout]: https://mfontanini.github.io/presenterm/guides/layout.html
[guide-latex]: https://mfontanini.github.io/presenterm/guides/latex.html
[guide-pdf-export]: https://mfontanini.github.io/presenterm/guides/pdf-export.html
[guide-key-bindings]: https://mfontanini.github.io/presenterm/guides/configuration.html#key-bindings
[guide-hot-reload]: https://mfontanini.github.io/presenterm/guides/basics.html#hot-reload
[bat]: https://github.com/sharkdp/bat
[syntect]: https://github.com/trishume/syntect


