# Introduction

This guide teaches you how to use _presenterm_. At this point you should have already installed _presenterm_, otherwise 
visit the [installation](/docs/install.md) guide to get started.

# Presentations

A presentation in _presenterm_ is a single markdown file. Every slide in the presentation file is delimited by a line 
that contains a single HTML comment:

```html
<!-- end_slide -->
```

Presentations can contain most commonly used markdown elements such as ordered and unordered lists, headings, formatted 
text (**bold**, _italics_, ~strikethrough~, `inline code`, etc), code blocks, block quotes, tables, etc.

## Images

Images are supported and will render in your terminal as long as it supports either the [iterm2 image 
protocol](https://iterm2.com/documentation-images.html), the [kitty graphics 
protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/), or [sixel](https://saitoha.github.io/libsixel/). Some of 
the terminals where at least one of these is supported are:

* kitty
* iterm2
* wezterm
* foot

Things you should know when using image tags in your presentation's markdown are:
* Image paths are relative to your presentation path. That is a tag like `![](food/potato.png)` will be looked up at 
  `$PRESENTATION_DIRECTORY/food/potato.png`.
* Images will be rendered in their original size. That is, if your terminal is 300x200px and your image is 200x100px, it 
  will take up 66% of your horizontal space and 50% of your vertical space.
* The exception to the point above is if the image does not fit in your terminal, it will be resized accordingly while 
  preserving the aspect ratio.
* If your terminal does not support any of the graphics protocol above, images will be rendered using ascii blocks. It 
  ain't great but it's something!

# Extensions

Besides the standard markdown elements, _presenterm_ supports a few extensions.

## Introduction slide

By setting a front matter at the beginning of your presentation, you can configure the title and author of your 
presentation and implicitly create an introduction slide:

```markdown
---
title: My first presentation
author: Myself
---
```

## Slide titles

Any [setext header](https://spec.commonmark.org/0.30/#setext-headings) will be considered to be a slide title and will 
be rendered in a more slide-title-looking way. By default this means it will be centered, some vertical padding will be 
added and the text color will be different.

~~~
Hello
===
~~~

> Note: see the [themes](/docs/themes.md) section on how to customize the looks of slide titles and any other element in 
> a presentation.

## Pauses

Pauses allow the sections of the content in your slide to only show up when you advance in your presentation. That is, 
only after you press, say, the right arrow will a section of the slide show up. This can be done by the `pause` comment 
command:

```html
<!-- pause -->
```

## Ending slides

While other applications use a thematic break (`---`) to mark the end of a slide, _presenterm_ uses a special 
`end_slide` HTML comment:

```html
<!-- end_slide -->
```

This makes the end of a slide more explicit and easy to spot while you're editing your presentation.

See the [configuration](/docs/config.md) if you want to customize this behavior.
