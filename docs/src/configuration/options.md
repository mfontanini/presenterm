# Options

Options are special configuration parameters that can be set either in the configuration file under the `options` key, 
or in a presentation's front matter under the same key. This last one allows you to customize a single presentation so 
that it acts in a particular way. This can also be useful if you'd like to share the source files for your presentation 
with other people.

The supported configuration options are currently the following:

## implicit_slide_ends

This option removes the need to use `<!-- end_slide -->` in between slides and instead assumes that if you use a slide 
title, then you're implying that the previous slide ended. For example, the following presentation:

```markdown
---
options:
  implicit_slide_ends: true
---

Tasty vegetables
================

* Potato

Awful vegetables
================

* Lettuce
```

Is equivalent to this "vanilla" one that doesn't use implicit slide ends.

```markdown
Tasty vegetables
================

* Potato

<!-- end_slide -->

Awful vegetables
================

* Lettuce
```

## end_slide_shorthand

This option allows using thematic breaks (`---`) as a delimiter between slides. When enabling this option, you can still 
use `<!-- end_slide -->` but any thematic break will also be considered a slide terminator.

```
---
options:
  end_slide_shorthand: true
---

this is a slide

---------------------

this is another slide
```

## command_prefix

Because _presenterm_ uses HTML comments to represent commands, it is necessary to make some assumptions on _what_ is a 
command and what isn't. The current heuristic is:

* If an HTML comment is laid out on a single line, it is assumed to be a command. This means if you want to use a real 
  HTML comment like `<!-- remember to say "potato" here -->`, this will raise an error.
* If an HTML comment is multi-line, then it is assumed to be a comment and it can have anything inside it. This means 
  you can't have a multi-line comment that contains a command like `pause` inside.

Depending on how you use HTML comments personally, this may be limiting to you: you cannot use any single line comments 
that are not commands. To get around this, the `command_prefix` option lets you configure a prefix that must be set in 
all commands for them to be configured as such. Any single line comment that doesn't start with this prefix will not be 
considered a command.

For example:

```
---
options:
  command_prefix: "cmd:"
---

<!-- remember to say "potato here" -->

Tasty vegetables
================

* Potato

<!-- cmd:pause -->

**That's it!**
```

In the example above, the first comment is ignored because it doesn't start with "cmd:" and the second one is processed 
because it does.

## incremental_lists

If you'd like all bullet points in all lists to show up with pauses in between you can enable the `incremental_lists` 
option:

```
---
options:
  incremental_lists: true
---

* pauses
* in
* between
```

Keep in mind if you only want specific bullet points to show up with pauses in between, you can use the 
[`incremental_lists` comment command](../features/commands.md#incremental-lists).

## strict_front_matter_parsing

This option tells _presenterm_ you don't care about extra parameters in presentation's front matter. This can be useful 
if you're trying to load a presentation made for another tool. The following presentation would only be successfully 
loaded if you set `strict_front_matter_parsing` to `false` in your configuration file:

```markdown
---
potato: 42
---

# Hi
```

## image_attributes_prefix

The [image size](../features/images.md#image-size) prefix (by default `image:`) can be configured to be anything you 
would want in case you don't like the default one. For example, if you'd like to set the image size by simply doing 
`![width:50%](path.png)` you would need to set:

```yaml
---
options:
  image_attributes_prefix: ""
---

![width:50%](path.png)
```

## auto_render_languages

This option allows indicating a list of languages for which the `+render` attribute can be omitted in their code 
snippets and will be implicitly considered to be set. This can be used for languages like `mermaid` so that graphs are 
always automatically rendered without the need to specify `+render` everywhere.

```yaml
---
options:
  auto_render_languages:
    - mermaid
---
```

## list_item_newlines

The option allows configuring the number of newlines in between list items, the default being `1`. This cam also be set 
via the `list_item_newlines` comment command.

```yaml
---
options:
    list_item_newlines: 2
---
```
