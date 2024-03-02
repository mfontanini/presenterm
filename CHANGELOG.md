# v0.7.0 - 2024-03-02

## New features

* Add color to prefix in block quote (#218).
* Allow having code blocks without background (#215 #216).
* Allow validating whether presentation overflows terminal (#209 #211).
* Add parameter to list themes (#207).
* Add catppuccin themes (#197 #205 #206) - thanks @Mawdac.
* Detect konsole terminal emulator (#204).
* Allow customizing slide title style (#201).

## Fixes

* Don't crash in present mode (#210).
* Set colors properly before displaying an error (#212).

## Improvements

* Suggest a tool is missing when spawning returns ENOTFOUND (#221).
* Sort input file list (#202) - thanks @bmwiedemann.
* Add more example presentations (#217).
* Add Scoop to package managers (#200) - thanks @nagromc.
* Remove support for uncommon image formats (#208).

# v0.6.1 - 2024-02-11

## Fixes

* Don't escape symbols in block quotes (#195).
* Respect `XDG_CONFIG_HOME` when loading configuration files and custom themes (#193).

# v0.6.0 - 2024-02-09

## Breaking changes

* The default configuration file and custom themes paths have been changed in Windows and macOS to be compliant to where 
  those platforms store these types of files. See the [configuration 
  guide](https://mfontanini.github.io/presenterm/guides/configuration.html) to learn more.

## New features

* Add `f` keys, tab, and backspace as possible bindings (#188).
* Add support for multiline block quotes (#184).
* Use theme color as background on ascii-blocks mode images (#182).
* Blend ascii-blocks image semi-transparent borders (#185).
* Respect Windows/macOS config paths for configuration (#181).
* Allow making front matter strict parsing optional (#190).

## Fixes

* Don't add an extra line after an end slide shorthand (#187).
* Don't clear input state on key release event (#183).

# v0.5.0 - 2024-01-26

## New features

* Support images on Windows (#120).
* Support animated gifs on kitty terminal (#157 #161).
* Support images on tmux running in kitty terminal (#166).
* Improve sixel support (#169 #172).
* Use synchronized updates to remove flickering when switching slides (#156).
* Add newlines command (#167).
* Detect image protocol instead of relying on viuer (#160).
* Turn documentation into mdbook (#141 #147) - thanks @pwnwriter.
* Allow using thematic breaks to end slides (#138).
* Allow specifying the preferred image protocol via `--image-protocol` / config file (#136 #170).
* Add slide index modal (#128 #139 #133 #158).
* Allow defining custom keybindings in config file (#132 #155).
* Add key bindings modal (#152).
* Prioritize CLI args `--theme` over anything else (#116).
* Allow enabling automatic list pauses (#106 #109 #110).
* Allow passing in config file path via CLI arg (#174).

## Fixes

* Shrink columns layout dimensions correctly when shrinking left (#113).
* Explicitly set execution output foreground color in built-in themes (#122).
* Detect sixel early and fallback to ascii blocks properly (#135).
* Exit with a clap error on missing path (#150).
* Don't blow up if presentation file temporarily disappears (#154).
* Parse front matter properly in presence of \r\n (#162).
* Don't preload graphics mode when generating pdf metadata (#168).
* Ignore key release events (#119).

## Improvements

* Validate that config file contains the right attributes (#107).
* Display first presentation load error as any other (#118).
* Add hashes for windows artifacts (#126).
* Remove arch packaging files (#111).
* Lower CPU and memory usage when displaying images (#157).

# v0.4.1 - 2023-12-22

## New features

* Cause an error if an unknown field name is found on a theme, config file, or front matter (#102).

## Fixes

* Explicitly disable kitty/iterm protocols when printing images in export PDF mode as this was causing PDF generation in 
  macOS to fail (#101).

# v0.4.0 - 2023-12-16

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
