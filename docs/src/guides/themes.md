## Themes

Themes are defined in the form of yaml files. A few built-in themes are defined in the [themes][builtin-themes] 
directory, but others can be created and referenced directly in every presentation.

### Setting themes

There's various ways of setting the theme you want in your presentation:

#### CLI

Passing in the `--theme` parameter when running _presenterm_ to select one of the built-in themes.

#### Within the presentation

The presentation's markdown file can contain a front matter that specifies the theme to use. This comes in 3 flavors:

##### By name

Using a built-in theme name makes your presentation use that one regardless of what the default or what the `--theme` 
option specifies:

```yaml
---
theme:
  name: dark
---
```

##### By path

You can define a theme file in yaml format somewhere in your filesystem and reference it within the presentation:

```yaml
---
theme:
  path: /home/me/Documents/epic-theme.yaml
---
```

##### Overrides

You can partially/completely override the theme in use from within the presentation:

```yaml
---
theme:
  override:
    default:
      colors:
        foreground: "beeeff"
---
```

This lets you:

1. Create a unique style for your presentation without having to go through the process of taking an existing theme, 
   copying somewhere, and changing it when you only expect to use it for that one presentation.
2. Iterate quickly on styles given overrides are reloaded whenever you save your presentation file.

## Built-in themes

A few built-in themes are bundled with the application binary, meaning you don't need to have any external files 
available to use them. These are packed as part of the [build process][build-rs] as a binary blob and are decoded on 
demand only when used.

Currently, the following themes are supported:

* `dark`: A dark theme.
* `light`: A light theme.
* `tokyonight-storm`: A theme inspired by the colors used in [toyonight](https://github.com/folke/tokyonight.nvim).
* `terminal-dark`: A theme that uses your terminals color and looks best if your terminal uses a dark color scheme. This 
  means if your terminal background is e.g. transparent, or uses an image, the presentation will inherit that.
* `terminal-light`: The same as `terminal-dark` but works best if your terminal uses a light color scheme.

## Loading custom themes

On startup, _presenterm_ will look into the `themes` directory under the [configuration directory](configuration.html) 
(e.g. `~/.config/presenterm/themes` in Linux) and will load any `.yaml` file as a theme and make it available as if it 
was a built-in theme. This means you can use it as an argument to the `--theme` parameter, use it in the `theme.name` 
property in a presentation's front matter, etc.

## Theme definition

This section goes through the structure of the theme files. Have a look at some of the [existing themes][builtin-themes] 
to have an idea of how to structure themes. 

### Root elements

The root attributes on the theme yaml files specify either:

* A specific type of element in the input markdown or rendered presentation. That is, the slide title, headings, footer, 
  etc.
* A default to be applied as a fallback if no specific style is specified for a particular element.

### Alignment

_presenterm_ uses the notion of alignment, just like you would have in a GUI editor, to align text to the left, center, 
or right. You probably want most elements to be aligned left, _some_ to be aligned on the center, and probably none to 
the right (but hey, you're free to do so!).

The following elements support alignment:
* Code blocks.
* Slide titles.
* The title, subtitle, and author elements in the intro slide.
* Tables.

#### Left/right alignment

Left and right alignments take a margin property which specifies the number of columns to keep between the text and the 
left/right terminal screen borders. 

The margin can be specified in two ways:

##### Fixed

A specific number of characters regardless of the terminal size.

```yaml
alignment: left
margin:
  fixed: 5
```

##### Percent

A percentage over the total number of columns in the terminal.

```yaml
alignment: left
margin:
  percent: 8
```

Percent alignment tends to look a bit nicer as it won't change the presentation's look as much when the terminal size 
changes.

#### Center alignment

Center alignment has 2 properties:
* `minimum_size` which specifies the minimum size you want that element to have. This is normally useful for code blocks 
  as they have a predefined background which you likely want to extend slightly beyond the end of the code on the right.
* `minimum_margin` which specifies the minimum margin you want, using the same structure as `margin` for left/right 
  alignment. This doesn't play very well with `minimum_size` but in isolation it specifies the minimum number of columns 
  you want to the left and right of your text.

### Colors

Every element can have its own background/foreground color using hex notation:

```yaml
default:
  colors:
    foreground: "ff0000"
    background: "00ff00"
```

### Default style

The default style specifies:

* The margin to be applied to all slides.
* The colors to be used for all text.

```yaml
default:
  margin:
    percent: 8
  colors:
    foreground: "e6e6e6"
    background: "040312"
```

### Intro slide

The introductory slide will be rendered if you specify a title, subtitle, or author in the presentation's front matter. 
This lets you have a less markdown-looking introductory slide that stands out so that it doesn't end up looking too 
monotonous:

```yaml
---
title: Presenting from my terminal
sub_title: Like it's 1990
author: John Doe
---
```

The theme can specify:
* For the title and subtitle, the alignment and colors.
* For the author, the alignment, colors, and positioning (`page_bottom` and `below_title`). The first one will push it 
  to the bottom of the screen while the second one will put it right below the title (or subtitle if there is one)

For example:

```yaml
intro_slide:
  title:
    alignment: left
    margin:
      percent: 8
  author:
    colors:
      foreground: black
    positioning: below_title
```

### Footer

The footer currently comes in 3 flavors:

#### None

No footer at all!

```yaml
footer:
  style: empty
```

#### Progress bar

A progress bar that will advance as you move in your presentation. This will by default use a block-looking character to 
draw the progress bar but you can customize it:

```yaml
footer:
  style: progress_bar

  # Optional!
  character: 🚀
```

#### Template

A template footer that lets you put something on the left, center and/or right of the screen. The template strings have 
access to `{author}` as specified in the front matter, `{current_slide}` and `{total_slides}` which will point to the 
current and total number of slides:

```yaml
footer:
  style: template
  left: "My name is {author}"
  center: @myhandle
  right: "{current_slide} / {total_slides}"
```

### Slide title

Slide titles, as specified by using a setext header, has the following properties:
* `padding_top` which specifies the number of rows you want as padding before the text.
* `padding_bottom` which specifies the number of rows you want as padding after the text.
* `separator` which specifies whether you want a horizontal ruler after the text (and the `padding_bottom`):

```yaml
slide_title:
  padding_bottom: 1
  padding_top: 1
  separator: true
```

### Headings

Every header type (h1 through h6) can have its own style composed of:
* The prefix you want to use.
* The colors, just like any other element:

```yaml
headings:
  h1:
    prefix: "██"
    colors:
      foreground: "rgb_(48,133,195)"
  h2:
    prefix: "▓▓▓"
    colors:
      foreground: "rgb_(168,223,142)"
```

### Code blocks

The syntax highlighting for code blocks is done via the [syntect](https://github.com/trishume/syntect) crate. The list 
of all the supported built-in _syntect_ themes is the following:

* base16-ocean.dark
* base16-eighties.dark
* base16-mocha.dark
* base16-ocean.light
* InspiredGitHub
* Solarized (dark)
* Solarized (light)

Besides those and thanks to the work done on the awesome [bat tool](https://github.com/sharkdp/bat), _presenterm_ has 
access to not only the built-in _syntect_'s built-in themes but also the ones in _bat_. Run `bat --list-themes` to see a 
list of all of them.

Code blocks can also have an optional vertical and horizontal padding so your code is not too close to its bounding 
rectangle:

```yaml
code:
  theme_name: base16-eighties.dark
  padding:
    horizontal: 2
    vertical: 1
```

#### Custom highlighting themes

Besides the built-in highlighting themes, you can drop any `.tmTheme` theme in the `themes/highlighting` directory under 
your [configuration directory](configuration.html) (e.g. `~/.config/presenterm/themes/highlighting` in Linux) and they 
will be loaded automatically when _presenterm_ starts.

### Block quotes

For block quotes you can specify a string to use as a prefix in every line of quoted text:

```yaml
block_quote:
  prefix: "▍ "
```

<!-- links -->
[builtin-themes]: https://github.com/mfontanini/presenterm/tree/master/themes
[build-rs]: https://github.com/mfontanini/presenterm/blob/master/build.rs
