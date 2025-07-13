# Settings

As opposed to options, the rest of these settings **can only be configured via the configuration file**.

## Default theme

The default theme can be configured only via the config file. When this is set, every presentation that doesn't set a 
theme explicitly will use this one:

```yaml
defaults:
  theme: light
```

## Terminal font size

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

## Preferred image protocol

By default _presenterm_ will try to detect which image protocol to use based on the terminal you are using. In case 
detection for some reason fails in your setup or you'd like to force a different protocol to be used, you can explicitly 
set this via the `--image-protocol` parameter or the configuration key `defaults.image_protocol`:

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

## Maximum presentation width

The `max_columns` property can be set to specify the maximum number of columns that the presentation will stretch to. If 
your terminal is larger than that, the presentation will stick to that size and will be centered, preventing it from 
looking too stretched.

```yaml
defaults:
  max_columns: 100
```

If you would like your presentation to be left or right aligned instead of centered when the terminal is too wide, you 
can use the `max_columns_alignment` key:

```yaml
defaults:
  max_columns: 100
  # Valid values: left, center, right
  max_columns_alignment: left
```

## Maximum presentation height

The `max_rows` and `max_rows_alignment` properties are analogous to `max_columns*` to allow capping the maximum number 
of rows:

```yaml
defaults:
  max_rows: 100
  # Valid values: top, center, bottom
  max_rows_alignment: left
```

## Incremental lists behavior

By default, [incremental lists](../features/commands.md) will pause before and after a list. If you would like to change 
this behavior, use the `defaults.incremental_lists` key:

```yaml
defaults:
  incremental_lists:
    # The defaults, change to false if desired.
    pause_before: true
    pause_after: true
```

# Slide transitions

Slide transitions allow animating your presentation every time you move from a slide to the next/previous one. The 
configuration for slide transitions is the following:

```yaml
transition:
  # how long the transition should last.
  duration_millis: 750

  # how many frames should be rendered during the transition
  frames: 45

  # the animation to use
  animation:
    style: <style_name>
```

See the [slide transitions page](../features/slide-transitions.md) for more information on which animation styles are 
supported.

# Key bindings

Key bindings that _presenterm_ uses can be manually configured in the config file via the `bindings` key. The following 
is the default configuration:

```yaml
bindings:
  # the keys that cause the presentation to move forwards.
  next: ["l", "j", "<right>", "<page_down>", "<down>", " "]

  # the keys that cause the presentation to move backwards.
  previous: ["h", "k", "<left>", "<page_up>", "<up>"]

  # the keys that cause the presentation to move "fast" to the next slide. this will ignore:
  #
  # * Pauses.
  # * Dynamic code highlights.
  # * Slide transitions, if enabled.
  next_fast: ["n"]

  # same as `next_fast` but jumps fast to the previous slide.
  previous_fast: ["p"]

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
  exit: ["<c-c>", "q"]

  # the key binding to suspend the application.
  suspend: ["<c-z>"]
```

You can choose to override any of them. Keep in mind these are overrides so if for example you change `next`, the 
default won't apply anymore and only what you've defined will be used.

# Snippet configurations

The configurations that affect code snippets in presentations.

## Snippet execution

[Snippet execution](../features/code/execution.md#executing-code-blocks) is disabled by default for security reasons. 
Besides passing in the `-x` command line parameter every time you run _presenterm_, you can also configure this globally 
for all presentations by setting:

```yaml
snippet:
  exec:
    enable: true
```

**Use this at your own risk**, especially if you're running someone else's presentations!

## Snippet execution + replace

[Snippet execution + replace](../features/code/execution.md#executing-and-replacing) is disabled by default for security 
reasons. Similar to `+exec`, this can be enabled by passing in the `-X` command line parameter or configuring it 
globally by setting:

```yaml
snippet:
  exec_replace:
    enable: true
```

**Use this at your own risk**. This will cause _presenterm_ to execute code without user intervention so don't blindly 
enable this and open a presentation unless you trust its origin!

## Custom snippet executors

If _presenterm_ doesn't support executing code snippets for your language of choice, please [create an 
issue](https://github.com/mfontanini/presenterm/issues/new)! Alternatively, you can configure this locally yourself by 
setting:

```yaml
snippet:
  exec:
    custom:
      # The keys should be the language identifier you'd use in a code block.
      c++:
        # The name of the file that will be created with your snippet's contents.
        filename: "snippet.cpp"

        # A list of environment variables that should be set before building/running your code.
        environment:
          MY_FAVORITE_ENVIRONMENT_VAR: foo

        # A prefix that indicates a line that starts with it should not be visible but should be executed if the
        # snippet is marked with `+exec`.
        hidden_line_prefix: "/// "

        # A list of commands that will be ran one by one in the same directory as the snippet is in.
        commands:
          # Compile if first
          - ["g++", "-std=c++20", "snippet.cpp", "-o", "snippet"]
          # Now run it 
          - ["./snippet"]
```

The output of all commands will be included in the code snippet execution output so if a command (like the `g++` 
invocation) was to emit any output, make sure to use whatever flags are needed to mute its output.

Also note that you can override built-in executors in case you want to run them differently (e.g. use `c++23` in the 
example above).

See more examples in the [executors.yaml](https://github.com/mfontanini/presenterm/blob/master/executors.yaml) file 
which defines all of the built-in executors. 

## Snippet rendering threads

Because some `+render` code blocks can take some time to be rendered into an image, especially if you're using 
[mermaid](https://mermaid.js.org/) charts, this is run asychronously. The number of threads used to render these, which 
defaults to 2, can be configured by setting:

```yaml
snippet:
  render:
    threads: 2
```

## Mermaid scaling

[mermaid](https://mermaid.js.org/) graphs will use a default scaling of `2` when invoking the mermaid CLI. If you'd like 
to change this use:


```yaml
mermaid:
  scale: 2
```

## D2 scaling

[d2](https://d2lang.com/) graphs will use the default scaling when invoking the d2 CLI. If you'd like to change this 
use:


```yaml
d2:
  scale: 2
```

## Enabling speaker note publishing

If you don't want to run _presenterm_ with `--publish-speaker-notes` every time you want to publish speaker notes, you 
can set the `speaker_notes.always_publish` attribute to `true`.

```yaml
speaker_notes:
  always_publish: true
```

# Presentation exports

The configurations that affect PDF and HTML exports.

## Export size

By default, the size of each page in the generated PDF and HTML files will depend on the size of your terminal. 

If you would like to instead configure the dimensions by hand, set the `export.dimensions` key:

```yaml
export:
  dimensions:
    columns: 80
    rows: 30
```

## Pause behavior

By default pauses will be ignored in generated PDF files. If instead you'd like every pause to generate a new page in 
the export, set the `export.pauses` attribute:

```yaml
export:
  pauses: new_slide
```

## Sequential snippet execution

When generating exports, snippets are executed in parallel to make the process faster. If your snippets require being 
executed sequentially, you can use the `export.snippets` parameter:

```yaml
export:
  snippets: sequential
```

## PDF font 

The PDF export can be configured to use a specific font installed in your system. Use the following keys to do so:

```yaml
export:
  pdf:
    fonts:
      normal: /usr/share/fonts/truetype/tlwg/TlwgMono.ttf
      italic: /usr/share/fonts/truetype/tlwg/TlwgMono-Oblique.ttf
      bold: /usr/share/fonts/truetype/tlwg/TlwgMono-Bold.ttf
      bold_italic: /usr/share/fonts/truetype/tlwg/TlwgMono-BoldOblique.ttf
```

