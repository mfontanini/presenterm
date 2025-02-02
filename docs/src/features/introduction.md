# Introduction

This guide teaches you how to use _presenterm_. At this point you should have already installed _presenterm_, otherwise 
visit the [installation](../installation.md) guide to get started.

## Quick start

Download the demo presentation and run it using:

```bash
git clone https://github.com/mfontanini/presenterm.git
cd presenterm
presenterm examples/demo.md
```

# Presentations

A presentation in _presenterm_ is a single markdown file. Every slide in the presentation file is delimited by a line 
that contains a single HTML comment:

```html
<!-- end_slide -->
```

Presentations can contain most commonly used markdown elements such as ordered and unordered lists, headings, formatted 
text (**bold**, _italics_, ~strikethrough~, `inline code`, etc), code blocks, block quotes, tables, etc.

## Introduction slide

By setting a front matter at the beginning of your presentation, you can configure the title, sub title, and author of 
your presentation and implicitly create an introduction slide:

```yaml
---
title: My first presentation
sub_title: (in presenterm!)
author: Myself
---
```

All of these attributes are optional so you're not forced to set them all.

### Multiple authors

If you're creating a presentation in which there's multiple authors, you can use the `authors` key instead of `author`
and list them all this way:

```yaml
---
title: Our first presentation
authors:
  - Me
  - You
---
```

## Slide titles

Any [setext header](https://spec.commonmark.org/0.30/#setext-headings) will be considered to be a slide title and will 
be rendered in a more slide-title-looking way. By default this means it will be centered, some vertical padding will be 
added and the text color will be different.

~~~markdown
Hello
===
~~~

> [!note]
> See the [themes](themes/introduction.md) section on how to customize the looks of slide titles and any other element 
> in a presentation.

## Ending slides

While other applications use a thematic break (`---`) to mark the end of a slide, _presenterm_ uses a special 
`end_slide` HTML comment:

```html
<!-- end_slide -->
```

This makes the end of a slide more explicit and easy to spot while you're editing your presentation. See the 
[configuration](../configuration/options.md#implicit_slide_ends) if you want to customize this behavior.

If you really would prefer to use thematic breaks (`---`) to delimit slides, you can do that by enabling the 
[`end_slide_shorthand`](../configuration/options.md#end_slide_shorthand) options.

## Colored text

`span` HTML tags can be used to provide foreground and/or background colors to text. Currently only the `style` 
attribute is supported, and only the CSS attributes `color` and `background-color` can be used to set the foreground and 
background colors respectively. Colors used in both CSS attributes can refer to [theme palette 
colors](themes/definition.md#color-palette) by using the `palette:<name>` or `p:<name` syntaxes.

For example, the following will use `ff0000` as the foreground color and whatever the active theme's palette defines as 
`foo`:

```markdown
<span style="color: #ff0000; background-color: palette:foo">colored text!</span>
```

> [!note]
> Keep in mind **only `span` tags are supported**.

# Key bindings

Navigation within a presentation should be intuitive: jumping to the next/previous slide can be done by using the arrow 
keys, _hjkl_, and page up/down keys.

Besides this:

* Jumping to the first slide: `gg`.
* Jumping to the last slide: `G`.
* Jumping to a specific slide: `<slide-number>G`.
* Exit the presentation: `<ctrl>c`.

You can check all the configured keybindings by pressing `?` while running _presenterm_.

## Configuring key bindings

If you don't like the default key bindings, you can override them in the [configuration 
file](../configuration/settings.md#key-bindings).

# Modals

_presenterm_ currently has 2 modals that can provide some information while running the application. Modals can be 
toggled using some key combination and can be hidden using the escape key by default, but these can be configured via 
the [configuration file key bindings](../configuration/settings.md#key-bindings).

## Slide index modal

This modal can be toggled by default using `control+p` and lets you see an index that contains a row for every slide in 
the presentation, including its title and slide index. This allows you to find a slide you're trying to jump to 
quicklier rather than scanning through each of them.

[![asciicast](https://asciinema.org/a/1VgRxVIEyLrMmq6OZ3oKx4PGi.svg)](https://asciinema.org/a/1VgRxVIEyLrMmq6OZ3oKx4PGi)

## Key bindings modal

The key bindings modal displays the key bindings for each of the supported actions and can be opened by pressing `?`.

# Hot reload

Unless you run in presentation mode by passing in the `--present` parameter, _presenterm_ will automatically reload your 
presentation file every time you save it. _presenterm_ will also automatically detect which specific slide was modified 
and jump to it so you don't have to be jumping back and forth between the source markdown and the presentation to see 
how the changes look like.

[![asciicast](https://asciinema.org/a/bu9ITs8KhaQK5OdDWnPwUYKu3.svg)](https://asciinema.org/a/bu9ITs8KhaQK5OdDWnPwUYKu3)
