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

