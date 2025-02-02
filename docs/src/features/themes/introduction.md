# Themes

_presenterm_ tries to be as configurable as possible, allowing users to create presentations that look exactly how they 
want them to look like. The tool ships with a set of [built-in 
themes](https://github.com/mfontanini/presenterm/tree/master/themes) but users can be created by users in their local 
setup and imported in their presentations.

## Setting themes

There's various ways of setting the theme you want in your presentation:

### CLI

Passing in the `--theme` parameter when running _presenterm_ to select one of the built-in themes.

### Within the presentation

The presentation's markdown file can contain a front matter that specifies the theme to use. This comes in 3 flavors:

#### By name

Using a built-in theme name makes your presentation use that one regardless of what the default or what the `--theme` 
option specifies:

```yaml
---
theme:
  name: dark
---
```

#### By path

You can define a theme file in yaml format somewhere in your filesystem and reference it within the presentation:

```yaml
---
theme:
  path: /home/me/Documents/epic-theme.yaml
---
```

#### Overrides

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

# Built-in themes

A few built-in themes are bundled with the application binary, meaning you don't need to have any external files 
available to use them. These are packed as part of the [build 
process](https://github.com/mfontanini/presenterm/blob/master/build.rs) as a binary blob and are decoded on demand only
when used.

Currently, the following themes are supported:

* `dark`: A dark theme.
* `light`: A light theme.
* `tokyonight-storm`: A theme inspired by the colors used in [toyonight](https://github.com/folke/tokyonight.nvim).
* A set of themes based on the [catppuccin](https://github.com/catppuccin/catppuccin) color palette:
  * `catppuccin-latte`
  * `catppuccin-frappe`
  * `catppuccin-macchiato`
  * `catppuccin-mocha`
* `terminal-dark`: A theme that uses your terminals color and looks best if your terminal uses a dark color scheme. This 
  means if your terminal background is e.g. transparent, or uses an image, the presentation will inherit that.
* `terminal-light`: The same as `terminal-dark` but works best if your terminal uses a light color scheme.

## Trying out built-in themes

All built-in themes can be tested by using the `--list-themes` parameter:

```bash
presenterm --list-themes
```

This will run a presentation where the same content is rendered using a different theme in each slide:

[![asciicast](https://asciinema.org/a/zeV1QloyrLkfBp6rNltvX7Lle.svg)](https://asciinema.org/a/zeV1QloyrLkfBp6rNltvX7Lle)

# Loading custom themes

On startup, _presenterm_ will look into the `themes` directory under the [configuration 
directory](../../configuration/introduction.md) (e.g. `~/.config/presenterm/themes` in Linux) and will load any `.yaml` 
file as a theme and make it available as if it was a built-in theme. This means you can use it as an argument to the 
`--theme` parameter, use it in the `theme.name` property in a presentation's front matter, etc.
