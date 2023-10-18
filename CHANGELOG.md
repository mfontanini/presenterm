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
