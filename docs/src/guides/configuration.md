## Configuration

_presenterm_ currently supports a limited number of configuration options that let you customize its behavior. Most 
configuration options can be set in two ways:

* Via the config file stored in `$HOME/.config/presenterm/config.yaml`. This file is not created automatically so you 
  will need to create it if you want to set configuration parameters there. Using this method will ensure all your 
  presentations use the same configuration.
* Via the front matter on a per presentation basis. This makes _that presentation_ use those configuration options while 
  leaving all others unaffected.

## Structure

Both in the front matter and in the config file, the structure of the configuration is the same:

```yaml
options:
  option1: value1
  option2: value2
  # ...
```

## Options

The supported configuration options are currently the following.

### implicit_slide_ends

This option removes the need to use `<!-- end_slide -->` in between slides and instead assumes that if you use a slide 
title, then you're implying that the previous slide ended. For example, the following presentation:

```markdown
---
options:
  implicit_slide_ends: true
---

Tasty vegetables
---

* Potato

Awful vegetables
---

* Lettuce
```

Is equivalent to this "vanilla" one that doesn't use implicit slide ends.

```markdown
Tasty vegetables
---

* Potato

<!-- end_slide -->

Awful vegetables
---

* Lettuce
```

### command_prefix

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

```markdown
---
options:
  command_prefix: "cmd:"
---

<!-- remember to say "potato here" -->

Tasty vegetables
---

* Potato

<!-- cmd:pause -->

**That's it!**
```

In the example above, the first comment is ignored because it doesn't start with "cmd:" and the second one is processed 
because it does.


## Default theme

The default theme can be configured only via the config file. When this is set, every presentation that doesn't set a 
theme explicitly will use this one:

```yaml
defaults:
  theme: light
```
