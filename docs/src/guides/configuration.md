## Configuration

_presenterm_ allows you to customize its behavior via a configuration file. This file is stored, along with all of your 
custom themes, in the following directories:

* `~/.config/presenterm/` in Linux.
* `~/Library/Application Support/presenterm/` in macOS.
* `~/AppData/Roaming/presenterm/config/` in Windows.

The configuration file will be looked up automatically in the directories above under the name `config.yaml`. e.g. on 
Linux you should create it under `~/.config/presenterm/config.yaml`. You can also specify a custom path to this file 
when running _presenterm_ via the `--config-path` parameter.

A [sample configuration file](https://github.com/mfontanini/presenterm/blob/master/config.sample.yaml) is provided in 
the repository that you can use as a base.

## Options

Options are special configuration parameters that can be set either in the configuration file under the `options` key, 
or in a presentation's front matter under the same key. This last one allows you to customize a single presentation so 
that it acts in a particular way. This can also be useful if you'd like to share the source files for your presentation 
with other people.

The supported configuration options are currently the following:

### implicit_slide_ends

This option removes the need to use `<!-- end_slide -->` in between slides and instead assumes that if you use a slide 
title, then you're implying that the previous slide ended. For example, the following presentation:

```
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

### end_slide_shorthand

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

### incremental_lists

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
[`incremental_lists` comment command](basics.html#incremental-lists).

### strict_front_matter_parsing

This option tells _presenterm_ you don't care about extra parameters in presentation's front matter. This can be useful 
if you're trying to load a presentation made for another tool. The following presentation would only be successfully 
loaded if you set `strict_front_matter_parsing` to `false` in your configuration file:

```markdown
---
potato: 42
---

# Hi
```

## Defaults

Defaults **can only be configured via the configuration file**.

### Default theme

The default theme can be configured only via the config file. When this is set, every presentation that doesn't set a 
theme explicitly will use this one:

```yaml
defaults:
  theme: light
```

### Terminal font size

This is a parameter that lets you explicitly set the terminal font size in use. This should not be used unless you are 
in Windows, given there's no (easy) way to get the terminal window size so we use this to figure out how large the 
window is and resize images properly. Some terminals on other platforms may also have this issue, but that should not be 
as common.

If you are on Windows or you notice images show up larger/smaller than they should, you can adjust this setting in your 
config file:

```yaml
defaults:
  terminal_font_size: 16
```

### Preferred image protocol

By default _presenterm_ will try to detect which image protocol to use based on the terminal you are using. In some 
cases this may fail, for example when using `tmux`. In those cases, you can explicitly set this via the 
`--image-protocol` parameter or the configuration key `defaults.image_protocol`:

```yaml
defaults:
  image_protocol: kitty-local
```

Possible values are:
* `auto`: try to detect it automatically (default).
* `kitty-local`: use the kitty protocol in "local" mode, meaning both _presenterm_ and the terminal run in the same host 
  and can share the filesystem to communicate.
* `kitty-remote`: use the kitty protocol in "remote" mode, meaning _presenterm_ and the terminal run in different hosts 
  and therefore can only communicate via terminal escape codes.
* `iterm2`: use the iterm2 protocol.
* `sixel`: use the sixel protocol. Note that this requires compiling _presenterm_ using the `--features sixel` flag.

## Key bindings

Key bindings that _presenterm_ uses can be manually configured in the config file via the `bindings` key. The following 
is the default configuration:

```yaml
bindings:
  # the keys that cause the presentation to move forwards.
  next: ["l", "j", "<right>", "<page_down>", "<down>", " "]

  # the keys that cause the presentation to move backwards.
  previous: ["h", "k", "<left>", "<page_up>", "<up>"]

  # the key binding to jump to the first slide.
  first_slide: ["gg"]

  # the key binding to jump to the last slide.
  last_slide: ["G"]

  # the key binding to jump to a specific slide.
  go_to_slide: ["<number>G"]

  # the key binding to execute a piece of shell code.
  execute_code: ["<c-e>"]

  # the key binding to reload the presentation.
  reload: ["<c-r>"]

  # the key binding to toggle the slide index modal.
  toggle_slide_index: ["<c-p>"] 

  # the key binding to toggle the key bindings modal.
  toggle_bindings: ["?"] 

  # the key binding to close the currently open modal.
  close_modal: ["<esc>"]

  # the key binding to close the application.
  exit: ["<c-c>"]
```

You can choose to override any of them. Keep in mind these are overrides so if for example you change `next`, the 
default won't apply anymore and only what you've defined will be used.
