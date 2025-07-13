# Comment commands

_presenterm_ uses "comment commands" in the form of HTML comments to let the user specify certain behaviors that can't 
be specified by vanilla markdown.

## Pauses

Pauses allow the sections of the content in your slide to only show up when you advance in your presentation. That is, 
only after you press, say, the right arrow will a section of the slide show up. This can be done by the `pause` comment 
command:

```html
<!-- pause -->
```

## Font size

The font size can be changed by using the `font_size` command:

```html
<!-- font_size: 2 -->
```

This causes the remainder of the slide to use the font size specified. The font size can range from 1 to 7, 1 being the 
default.

> [!note]
> This is currently only supported in the [_kitty_](https://sw.kovidgoyal.net/kitty/) terminal and only as of version 
> 0.40.0. See the notes on font sizes on the [introduction page](introduction.md#font-sizes) for more information on 
> this.

## Jumping to the vertical center

The command `jump_to_middle` lets you jump to the middle of the page vertically. This is useful in combination
with slide titles to create separator slides:

```markdown
blablabla

<!-- end_slide -->

<!-- jump_to_middle -->

Farming potatoes
===

<!-- end_slide -->
```

This will create a slide with the text "Farming potatoes" in the center, rendered using the slide title style.

## Explicit new lines

The `newline`/`new_line` and `newlines`/`new_lines` commands allow you to explicitly create new lines. Because markdown 
ignores multiple line breaks in a row, this is useful to create some spacing where necessary:

```markdown
hi

<!-- new_lines: 10 -->

mom

<!-- new_line -->

bye
```

## Incremental lists

Using `<!-- pause -->` in between each bullet point a list is a bit tedious so instead you can use the 
`incremental_lists` command to tell _presenterm_ that **until the end of the current slide** you want each individual 
bullet point to appear only after you move to the next slide:

```markdown
<!-- incremental_lists: true -->

* this
* appears
* one after
* the other

<!-- incremental_lists: false -->

* this appears
* all at once
```

## Number of lines in between list items

The `list_item_newlines` option lets you configure the number of new lines in between list items in the remainder of a 
slide. This can be helpful to "unpack" a list that only has a few entries and you want it to take up more space in a 
slide. This can also be configured for all lists via the [`options.list_item_newlines` 
option](../configuration/options.md#list_item_newlines).

```markdown
<!-- list_item_newlines: 2 -->

* this
* is
* more
* spaced
```

## Including external markdown files

By using the `include` command you can include the contents of an external markdown file as if it was part of the 
original presentation file:

```markdown
<!-- include: foo.md -->
```

Any files referenced by an included file will have their paths relative to that path. e.g. if you include `foo/bar.md` 
and that file contains an image `tar.png`, that image will be looked up in `foo/tar.png`.

## No footer

If you don't want the footer to show up in some particular slide for some reason, you can use the `no_footer` command:

```html
<!-- no_footer -->
```

## Skip slide

If you don't want a specific slide to be included in the presentation use the `skip_slide` command:

```html
<!-- skip_slide -->
```

## Text alignment

The text alignment for the remainder of the slide can be configured via the `alignment` command, which can use values: 
`left`, `center`, and `right`:

```markdown
<!-- alignment: left -->

left alignment, the default

<!-- alignment: center -->

centered

<!-- alignment: right -->

right aligned
```

