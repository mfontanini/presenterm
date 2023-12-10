presenterm
---

A terminal based slideshow tool.

---

_presenterm_ lets you define slideshows that run in your terminal.

This is how the [demo presentation](examples/demo.md) looks like:

![](assets/demo.gif)

# Installation

The recommended way to install _presenterm_ is to download the latest pre-built version for your system from the 
[releases](https://github.com/mfontanini/presenterm/releases) page.

For other alternative ways to install it, visit the [installation](/docs/install.md) docs.

# Features

* Define your presentation in a single markdown file.
* Image rendering support for _iterm2_, terminals that support the _kitty_ graphics protocol, and _sixel_.
* [Customize your presentation's look](/docs/themes.md) by defining themes, including colors, margins, layout 
  (left/center aligned content), footer for every slide, etc.
* Code highlighting for a [wide list of programming languages](/docs/highlighting.md).
* [Selective/dynamic](docs/highlighting.md) code highlighting such that only a subset of the lines are highlighted at a 
  time, and different sets of lines are highlighted as you move your slides forward.
* Configurable [column layouts](/docs/layouts.md) that let you structure parts of your slide into columns.
* Support for [_LaTeX_ and _typst_ code block rendering](/docs/latex.md) so you can define formulas as text and have 
  them automatically render as images when rendering a presentation.
* Support for an introduction slide that displays the presentation title and your name.
* Support for slide titles.
* Support for [shell code execution](docs/highlighting.md).
* Support for generating PDF files from presentations to share with other people.
* Create pauses in between each slide so that it progressively renders for a more interactive presentation.
* Automatically reload your presentation every time it changes for a fast development loop.
* [Terminal colored themes](/docs/themes.md) that use your terminal's color scheme. This means if your terminal uses 
  transparent backgrounds or images as background, those will be implicitly set as the background of your presentation.

## Hot reload

Unless you run in presentation mode by passing in the `--present` parameter, _presenterm_ will automatically reload your 
presentation file every time you save it. _presenterm_ will also automatically detect which specific slide was modified 
and jump to it so you don't have to be jumping back and forth between the source markdown and the presentation to see 
how the changes look like.

[![asciicast](https://asciinema.org/a/UTestkjb8M8K2mQgf9rDmzDGA.svg)](https://asciinema.org/a/UTestkjb8M8K2mQgf9rDmzDGA)

## Slides

Every slide must be separated by an HTML comment:

```html
<!-- end_slide -->
```

This makes it explicit that you want to end the current slide. Other tools use `---` instead which is less explicit and 
also is a valid markdown element which you may use in your presentation.

## Pauses

Just like [lookatme](https://github.com/d0c-s4vage/lookatme) does, _presenterm_ allows pauses in between your slide. 
This lets you have more interactive presentations where pieces of it start popping up as you move forward through it.

Similar to slide delimiters, pauses can be created by using the following HTML comment:

```html
<!-- pause -->
```

## Images

Images are supported if you're using iterm2, a terminal the supports the kitty graphics protocol (such as 
[kitty](https://sw.kovidgoyal.net/kitty/), of course), or one that supports sixel. sixel support requires building 
_presenterm_ with the `sixel` feature flag, which is disabled by default. You can do this by passing in the `--features sixel` parameters when running `cargo build`:

```shell
cargo build --release --features sixel
```

> **Note**: this feature flag is only needed if your terminal emulator only supports sixel. Many terminals support the kitty or iterm2 protocols so this isn't necessary.

Images are rendered **in their default size**. This means if your terminal window is 100 pixels wide and your image is 
50 pixels wide, it will take up 50% of the width. If an image does not fit in the screen, it will be scaled down to fit 
it.

![](assets/demo-image.png)

> **Note**: image rendering is currently not supported on Windows.

## Themes

_presenterm_ supports themes so you can customize your presentation's look. See the [built-in themes](themes) as 
examples on how to customize them.

You can define your own themes and make your presentation use it or you can also customize a theme within your 
presentation by including a front matter at the beginning of your presentation file:

```yaml
---
theme:
  # Specify it by name for built-in themes
  name: my-favorite-theme

  # Otherwise specify the path for it
  path: /home/myself/themes/epic.yaml

  # Or override parts of the theme right here
  override:
    default:
      colors:
        foreground: "beeeff"
---
```

Note that if you're in the default hot reload mode, overriding your theme will result in those changes being immediately 
applied to your presentation. This lets you easily test out color schemes quickly without having to close and reopen the 
application.

See the [documentation](/docs/themes.md) on themes to learn more.

## Introduction slide

By including a `title`, `sub_title` and/or `author` attribute in your front matter, you can create an introduction slide 
at the beginning of your presentation to display those:

```yaml
---
title: My first presentation
sub_title: (in presenterm!)
author: John Doe
---
```

## Slide titles

By using [setext headers](https://spec.commonmark.org/0.20/#setext-headers) you can create slide titles. These allow you 
to have a more slide-title-looking slide titles than using regular markdown headers:

```markdown
My slide title
---
```

> Note: nothing prevents you from using setext headers somewhere in the middle of a slide, which will make them render 
> as slide titles. Not sure why you'd want that but hey, you're free to do so!

## Column layouts

Column layouts allow you to organize content into columns. You can define 2 or more columns, choose how wide you want 
them to be, and then put any content into them. For example:

![](/assets/layouts.png)

See the [documentation](/docs/layouts.md) on layouts to learn more.

## Shell code execution

Any shell code can be marked for execution, making  _presenterm_ execute it and render its output when you press ctrl+e. 
In order to do this, annotate the code block with `+exec` (e.g. `bash +exec`). **Obviously use this at your own risk!**

[![asciicast](https://asciinema.org/a/1v3IqCEtU9tqDjVj78Pp7SSe2.svg)](https://asciinema.org/a/1v3IqCEtU9tqDjVj78Pp7SSe2)

See more details on this [here](docs/highlighting.md).

## PDF export

Presentations can be converted into PDF by using a helper tool. You can install it by running:

```shell
pip install presenterm-export
```

The only external dependency you'll need is [tmux](https://github.com/tmux/tmux/). After you've installed both of these, 
simply run _presenterm_ with the `--export-pdf` parameter to generate the output PDF:

```shell
presenterm --export-pdf examples/demo.md
```

The output PDF will be placed in `examples/demo.pdf`. The size of each page will depend on the size of your terminal so 
make sure to adjust accordingly before running the command above.

> Note: if you're using a separate virtual env to install _presenterm-export_ just make sure you activate it before 
> running _presenterm_ with the `--export-pdf` parameter.

## Navigation

Navigation should be intuitive: jumping to the next/previous slide can be done by using the arrow, _hjkl_, and page 
up/down keys.

Besides this:

* Jumping to the first slide: `gg`.
* Jumping to the last slide: `G`.
* Jumping to a specific slide: `<slide-number>G`.
* Exit the presentation: `<ctrl>c`.
* Refresh images: `<ctrl>r`.

# Docs

Some docs on how to configure _presenterm_ and how it works internally can be found [here](docs/README.md).

# Acknowledgements

This tool is heavily inspired by:

* [slides](https://github.com/maaslalani/slides/)
* [lookatme](https://github.com/d0c-s4vage/lookatme).
* [sli.dev](https://sli.dev/).

Support for code highlighting on many languages is thanks to [bat](https://github.com/sharkdp/bat), which contains a 
custom set of syntaxes that extend [syntect](https://github.com/trishume/syntect)'s default set of supported languages. 
Run `presenterm --acknowledgements` to get a full list of all the licenses for the binary files being pulled in.


