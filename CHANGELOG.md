# v0.4.0 - TODOOOOOO

## New features

* Add support for all of [bat](https://github.com/sharkdp/bat)'s code highlighting themes (#67).
* Add `terminal-dark` and `terminal-light` themes that preserve the terminal's colors and background (#68 #69).
* Allow placing themes in `$HOME/.config/presenterm/themes` to make them available automatically as if they were 
  built-in themes (#73).
* Allow configuring the default theme in `$HOME/.config/presenterm/config.yaml` (#74).
* Add support for rendering _LaTeX_ and _typst_ code blocks automatically as images (#75 #76 #79 #81).
* Add syntax highlighting support for _nix_ and _diff_ (#78 #82).
* Add comment command to jump into the middle of a slide (#86).
* Add configuration option to have implicit slide ends (#87 #89).
* Add configuration option to have custom comment-command prefix (#91).

# v0.3.0 - 2023-11-24

## New features

* Support more languages in code blocks thanks to [bat](https://github.com/sharkdp/bat)'s syntax sets (#21 #53).
* Add shell script executable code blocks (#17).
* Allow exporting presentation to PDF (#43 #60).
* Pauses no longer create new slides (#18 #25 #34 #42).
* Allow display code block line numbers (#46).
* Allow code block selective line highlighting (#48).
* Allow code block dynamic line highlighting (#49).
* Support animated gifs when using the iterm2 image protocol (#56).
* Nix flake packaging (#11 #27).
* Arch repo packaging (#10).
* Ignore vim-like code folding tags in comments.
* Add keybinding to refresh assets in presentation (#38).
* Template style footer is now one row above bottom (#39).
* Add `light` theme.

## Fixes

* Don't crash on Windows when terminal window size can't be found (#14).
* Don't reset numbers on ordered lists when using pauses in between (#19).
* Show proper line number when parsing a comment command fails (#29 #40).
* Don't reset the default footer when overriding theme in presentation without setting footer (#52).
* Don't let code blocks/block quotes that don't fit on the screen cause images to overlap with text (#57).

# v0.2.1 - 2023-10-18

## New features

* Binary artifacts are now automatically generated when a new release is done (#5) - thanks @pwnwriter.

# v0.2.0 - 2023-10-17

## New features

* [Column layouts](https://github.com/mfontanini/presenterm/blob/26e2eb28884675aac452f4c6e03f98413654240c/docs/layouts.md) that let you structure slides into columns.
* Support for `percent` margin rather than only a fixed number of columns.
* Spacebar now moves the presentation into the next slide.
* Add support for `center` footer when using the `template` mode.
* **Breaking**: themes now only use colors in hex format.

## Fixes

* Allow using `sh` as language for code block (#3).
* Minimum size for code blocks is now prioritized over minimum margin.
* Overflowing lines in lists will now correctly be padded to align all text under the same starting column.
* Running `cargo run` will now rebuild the tool if any of the built-in themes changed.
* `alignment` was removed from certain elements (like `list`) as it didn't really make sense.
* `default.alignment` is now no longer supported and by default we use left alignment. Use `default.margin` to specify the margins to use.

# v0.1.0 - 2023-10-08

## Features
* Define your presentation in a single markdown file.
* Image rendering support for iterm2, terminals that support the kitty graphics protocol, or sixel.
* Customize your presentation's look by defining themes, including colors, margins, layout (left/center aligned 
  content), footer for every slide, etc.
* Code highlighting for a wide list of programming languages.
* Support for an introduction slide that displays the presentation title and your name.
* Support for slide titles.
* Create pauses in between each slide so that it progressively renders for a more interactive presentation.
* Text formatting support for **bold**, _italics_, ~strikethrough~, and `inline code`.
* Automatically reload your presentation every time it changes for a fast development loop.
