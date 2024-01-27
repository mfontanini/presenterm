presenterm
===

**_presenterm_ lets you create presentations in markdown format and render them in your terminal.**

This is how the [demo presentation](examples/demo.md) looks like:

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

See the [introduction page][guide-basics] to learn more.

# Acknowledgements

This tool is heavily inspired by:

* [slides](https://github.com/maaslalani/slides/)
* [lookatme](https://github.com/d0c-s4vage/lookatme).
* [sli.dev](https://sli.dev/).

Support for code highlighting on many languages is thanks to [bat](https://github.com/sharkdp/bat), which contains a 
custom set of syntaxes that extend [syntect](https://github.com/trishume/syntect)'s default set of supported languages. 
Run `presenterm --acknowledgements` to get a full list of all the licenses for the binary files being pulled in.

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

