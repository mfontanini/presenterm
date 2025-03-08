# v0.11.0 - 2025-03-08

## Breaking changes

* Footer templates are now sanitized, and any variables surrounded in braces that aren't supported (e.g. `{potato}`) will now cause _presenterm_ to display an error. If you'd like to use braces in contexts where you're not trying to reference a variable you can use double braces, e.g. `live at {{PotatoConf}}` ([#442](https://github.com/mfontanini/presenterm/issues/442)) ([#467](https://github.com/mfontanini/presenterm/issues/467)) ([#469](https://github.com/mfontanini/presenterm/issues/469)) ([#471](https://github.com/mfontanini/presenterm/issues/471)).

## New features

* [Add support for kitty's font size protocol](https://mfontanini.github.io/presenterm/features/introduction.html#font-sizes). This is now used by default in built in themes in a few components such as the intro slide's title and slide titles. See the [example presentation gif](https://github.com/mfontanini/presenterm/blob/master/docs/src/assets/demo.gif) to check out how this looks like. Terminal suport for this feature is detected on startup and will be ignored if unsupported. This requires _kitty_ >= 0.40.0 ([#438](https://github.com/mfontanini/presenterm/issues/438)) ([#460](https://github.com/mfontanini/presenterm/issues/460)) ([#470](https://github.com/mfontanini/presenterm/issues/470)).
* [Allow specifying font size in a comment command](https://mfontanini.github.io/presenterm/features/commands.html#font-size), which causes any subsequent text in a slide to use the specified font size. Just like the above, only supported in _kitty_ >= 0.40.0 for now ([#458](https://github.com/mfontanini/presenterm/issues/458)).
* [Footers can now contain images](https://mfontanini.github.io/presenterm/features/themes/definition.html#footer-images) in the left and center components. This allows including some form of branding/company logo to your presentations ([#450](https://github.com/mfontanini/presenterm/issues/450)) ([#476](https://github.com/mfontanini/presenterm/issues/476)).
* [Footers can now contain inline markdown](https://mfontanini.github.io/presenterm/features/themes/definition.html#template-footers), which allows using bold, italics, `<span>` tags for colors, etc ([#466](https://github.com/mfontanini/presenterm/issues/466)).
* [Presentation titles can now contain inline markdown](https://mfontanini.github.io/presenterm/features/introduction.html#introduction-slide) ([#464](https://github.com/mfontanini/presenterm/issues/464)).
* [Introduce palette.classes in themes](https://mfontanini.github.io/presenterm/features/themes/definition.html#color-palette) to allow specifying combinations of foreground/background colors that can be referenced via the `class` attribute in `<span>` tags ([#468](https://github.com/mfontanini/presenterm/issues/468)).
* It's now possible to [configure the alignment](https://mfontanini.github.io/presenterm/configuration/settings.html#maximum-presentation-width) to use when `max_columns` is configured and the terminal width is larger than it ([#475](https://github.com/mfontanini/presenterm/issues/475)).
* Add support for wikilinks ([#448](https://github.com/mfontanini/presenterm/issues/448)).

## Fixes

* Don't get stuck if tmux doesn't passthrough ([#456](https://github.com/mfontanini/presenterm/issues/456)).
* Don't squash image if terminal's font aspect ratio is not 2:1 ([#446](https://github.com/mfontanini/presenterm/issues/446)).
* Fail if `--config-file` points to non existent file ([#474](https://github.com/mfontanini/presenterm/issues/474)).
* Use right script name for kotlin files when executing ([#462](https://github.com/mfontanini/presenterm/issues/462)).
* Respect lists that start at non 1 indexes ([#459](https://github.com/mfontanini/presenterm/issues/459)).
* Jump to right slide on code attribute change ([#478](https://github.com/mfontanini/presenterm/issues/478)).

## Improvements

* Remove `result` return type from builder fns that don't need it ([#465](https://github.com/mfontanini/presenterm/issues/465)).
* Refactor theme code ([#463](https://github.com/mfontanini/presenterm/issues/463)).
* Restructure `terminal` code and add test for margins/layouts ([#443](https://github.com/mfontanini/presenterm/issues/443)).
* Use `fastrand` instead of `rand` ([#441](https://github.com/mfontanini/presenterm/issues/441)).
* Avoid cloning strings when styling them ([#440](https://github.com/mfontanini/presenterm/issues/440)).

# v0.10.1 - 2025-02-14

## Fixes

* Don't error out if `options` in front matter doesn't include `auto_render_languages` ([#454](https://github.com/mfontanini/presenterm/pull/454)).
* Bump sixel-rs to 0.4.1 to fix build in aarch64 and riscv64 ([#452](https://github.com/mfontanini/presenterm/pull/452)) - thanks @Xeonacid.

# v0.10.0 - 2025-02-02

## New features

* Support for presentation speaker notes ([#389](https://github.com/mfontanini/presenterm/issues/389)) ([#419](https://github.com/mfontanini/presenterm/issues/419)) ([#421](https://github.com/mfontanini/presenterm/issues/421)) ([#425](https://github.com/mfontanini/presenterm/issues/425)) - thanks @dmackdev.
* Add support for colored text via inline `span` HTML tags  ([#390](https://github.com/mfontanini/presenterm/issues/390)).
* Add a color palette in themes to allow reusing colors across the theme and using predefined colors inside `span` tags ([#427](https://github.com/mfontanini/presenterm/issues/427)).
* Add support for github/gitlab style markdown alerts ([#423](https://github.com/mfontanini/presenterm/issues/423)) ([#430](https://github.com/mfontanini/presenterm/issues/430)).
* Allow using `+image` on code blocks to consume their output as an image ([#429](https://github.com/mfontanini/presenterm/issues/429)).
* Allow multiline comment commands ([#424](https://github.com/mfontanini/presenterm/issues/424)).
* Allow auto rendering mermaid/typst/latex code blocks ([#418](https://github.com/mfontanini/presenterm/issues/418)).
* Allow capping max columns on presentation ([#417](https://github.com/mfontanini/presenterm/issues/417)).
* Automatically detect kitty support, including when running inside tmux ([#406](https://github.com/mfontanini/presenterm/issues/406)).
* Use kitty image protocol in ghostty ([#405](https://github.com/mfontanini/presenterm/issues/405)).
* Force color output in rust, c, and c++ compiler executions ([#401](https://github.com/mfontanini/presenterm/issues/401)).
* Add graphql code highlighting ([#385](https://github.com/mfontanini/presenterm/issues/385)) - thanks @GV14982.
* Add tcl code highlighting ([#387](https://github.com/mfontanini/presenterm/issues/387)) - thanks @jtplaarj.
* Add Haskell executor ([#414](https://github.com/mfontanini/presenterm/issues/414)) - thanks @feature-not-a-bug.
* Add C# to code executors ([#399](https://github.com/mfontanini/presenterm/issues/399)) - thanks @giggio.
* Add R to executors ([#393](https://github.com/mfontanini/presenterm/issues/393)) - thanks @jonocarroll.

## Fixes

* Check for `term_program` before `term` to determine emulator ([#420](https://github.com/mfontanini/presenterm/issues/420)).
* Allow jumping back to column in column layout ([#396](https://github.com/mfontanini/presenterm/issues/396)).
* Ignore comments that start with `vim:` prefix ([#395](https://github.com/mfontanini/presenterm/issues/395)).
* Respect `+no_background` on a `+exec_replace` block ([#383](https://github.com/mfontanini/presenterm/issues/383)).

## Docs

* Document tmux active session bug ([#402](https://github.com/mfontanini/presenterm/issues/402)).
* Add notes on running `bat` directly ([#397](https://github.com/mfontanini/presenterm/issues/397)).

# v0.9.0 - 2024-10-06

## Breaking changes

* Default themes now no longer use a progress bar based footer. Instead they use indicator of the current page number 
and the total number of pages. If you'd like to preserve the old behavior, you can override the theme by using 
`footer.style = progress_bar` in [your 
theme](https://mfontanini.github.io/presenterm/guides/themes.html#setting-themes).
* Links that include a title (e.g. `[my title](http://example.com)`) now have their title rendered as well. Removing a 
link's title will make it look the same as they used to.

## New features

* Use "template" footer in built-in themes ([#358](https://github.com/mfontanini/presenterm/issues/358)).
* Allow including external code snippets ([#328](https://github.com/mfontanini/presenterm/issues/328)) 
  ([#372](https://github.com/mfontanini/presenterm/issues/372)).
* Add `+no_background` property to remove background from code blocks 
  ([#363](https://github.com/mfontanini/presenterm/issues/363)) 
  ([#368](https://github.com/mfontanini/presenterm/issues/368)).
* Show colored output from snippet execution output ([#316](https://github.com/mfontanini/presenterm/issues/316)).
* Style markdown inside block quotes ([#350](https://github.com/mfontanini/presenterm/issues/350)) 
  ([#351](https://github.com/mfontanini/presenterm/issues/351)).
* Allow using all intro slide variables in footer template 
  ([#338](https://github.com/mfontanini/presenterm/issues/338)).
* Include hidden line prefix in executors file ([#337](https://github.com/mfontanini/presenterm/issues/337)).
* Show link labels and titles ([#334](https://github.com/mfontanini/presenterm/issues/334)).
* Add `+exec_replace` which executes snippets and replaces them with their output 
  ([#330](https://github.com/mfontanini/presenterm/issues/330)) 
  ([#371](https://github.com/mfontanini/presenterm/issues/371)).
* Always show snippet execution bar ([#329](https://github.com/mfontanini/presenterm/issues/329)).
* Handle suspend signal (SIGTSTP) ([#318](https://github.com/mfontanini/presenterm/issues/318)).
* Allow closing with `q` ([#321](https://github.com/mfontanini/presenterm/issues/321)).
* Add event, location, and date labels in intro slide ([#317](https://github.com/mfontanini/presenterm/issues/317)).
* Use transparent background in mermaid charts ([#314](https://github.com/mfontanini/presenterm/issues/314)).
* Add `+acquire_terminal` to acquire the terminal when running snippets 
  ([#366](https://github.com/mfontanini/presenterm/issues/366))
  ([#376](https://github.com/mfontanini/presenterm/pull/376)).
* Add PHP executor ([#332](https://github.com/mfontanini/presenterm/issues/332)).
* Add Racket syntax highlighting ([#367](https://github.com/mfontanini/presenterm/issues/367)).
* Add TOML highlighting ([#361](https://github.com/mfontanini/presenterm/issues/361)).

## Fixes

* Wrap code snippets if they don't fit in terminal ([#320](https://github.com/mfontanini/presenterm/issues/320)).
* Allow list-themes/acknowledgements to run without path ([#359](https://github.com/mfontanini/presenterm/issues/359)).
* Translate tabs in code snippets to 4 spaces ([#356](https://github.com/mfontanini/presenterm/issues/356)).
* Add padding to right of code block wrapped lines ([#354](https://github.com/mfontanini/presenterm/issues/354)).
* Don't wrap code snippet separator line ([#353](https://github.com/mfontanini/presenterm/issues/353)).
* Show block quote prefix when wrapping ([#352](https://github.com/mfontanini/presenterm/issues/352)).
* Don't crash on code block with only hidden-line-prefixed lines 
  ([#347](https://github.com/mfontanini/presenterm/issues/347)).
* Canonicalize resources path ([#333](https://github.com/mfontanini/presenterm/issues/333)).
* Execute script relative to current working directory ([#323](https://github.com/mfontanini/presenterm/issues/323)).
* Support rendering mermaid charts on windows ([#319](https://github.com/mfontanini/presenterm/issues/319)).

## Improvements

* Add example on how column layouts and pauses interact ([#348](https://github.com/mfontanini/presenterm/issues/348)).
* Rename `jump_to_vertical_center` -> `jump_to_middle` in docs 
  ([#342](https://github.com/mfontanini/presenterm/issues/342)).
* Document `all` snippet highlighting keyword ([#335](https://github.com/mfontanini/presenterm/issues/335)).

# v0.8.0 - 2024-07-29

## Breaking changes

* Force users to explicitly enable snippet execution ([#276](https://github.com/mfontanini/presenterm/issues/276)) ([#281](https://github.com/mfontanini/presenterm/issues/281)).

## New features

* Code snippet execution for various programming languages ([#253](https://github.com/mfontanini/presenterm/issues/253)) ([#255](https://github.com/mfontanini/presenterm/issues/255)) ([#256](https://github.com/mfontanini/presenterm/issues/256)) ([#258](https://github.com/mfontanini/presenterm/issues/258)) ([#282](https://github.com/mfontanini/presenterm/issues/282)).
* Allow executing compiled snippets in windows ([#303](https://github.com/mfontanini/presenterm/issues/303)).
* Add support for hidden lines in code snippets ([#283](https://github.com/mfontanini/presenterm/issues/283)) ([#254](https://github.com/mfontanini/presenterm/issues/254)) - thanks @dmackdev.
* Support [mermaid](https://mermaid.js.org/) snippet rendering to image via `+render` attribute ([#268](https://github.com/mfontanini/presenterm/issues/268)).
* Allow scaling images dynamically based on terminal size ([#288](https://github.com/mfontanini/presenterm/issues/288)) ([#291](https://github.com/mfontanini/presenterm/issues/291)).
* Allow scaling images generated via `+render` code blocks (mermaid, typst, latex) ([#290](https://github.com/mfontanini/presenterm/issues/290)).
* Show `stderr` output from code execution ([#252](https://github.com/mfontanini/presenterm/issues/252)) - thanks @dmackdev.
* Wait for code execution process to exit completely ([#250](https://github.com/mfontanini/presenterm/issues/250)) - thanks @dmackdev.
* Generate images in `+render` code snippets asynchronously ([#273](https://github.com/mfontanini/presenterm/issues/273)) ([#293](https://github.com/mfontanini/presenterm/issues/293)) ([#284](https://github.com/mfontanini/presenterm/issues/284)) ([#279](https://github.com/mfontanini/presenterm/issues/279)).
* Dim non highlighted code snippet lines ([#287](https://github.com/mfontanini/presenterm/issues/287)).
* Shrink snippet execution to match code block width ([#286](https://github.com/mfontanini/presenterm/issues/286)).
* Include code snippet execution output in generated PDF ([#295](https://github.com/mfontanini/presenterm/issues/295)).
* Cache `+render` block images ([#270](https://github.com/mfontanini/presenterm/issues/270)).
* Add kotlin script executor ([#257](https://github.com/mfontanini/presenterm/issues/257)) - thanks @dmackdev.
* Add nushell code execution ([#274](https://github.com/mfontanini/presenterm/issues/274)) ([#275](https://github.com/mfontanini/presenterm/issues/275)) - thanks @PitiBouchon.
* Add rust-script as a new code executor ([#269](https://github.com/mfontanini/presenterm/issues/269)) - @ZhangHanDong. 
* Allow custom themes to extend others ([#265](https://github.com/mfontanini/presenterm/issues/265)).
* Allow jumping fast between slides ([#244](https://github.com/mfontanini/presenterm/issues/244)).
* Allow explicitly disabling footer in certain slides ([#239](https://github.com/mfontanini/presenterm/issues/239)).
* Allow using image paths in typst ([#235](https://github.com/mfontanini/presenterm/issues/235)).
* Add JSON schema for validation,completion,documentation ([#228](https://github.com/mfontanini/presenterm/issues/228)) ([#236](https://github.com/mfontanini/presenterm/issues/236)) - thanks @mikavilpas.
* Allow having multiple authors ([#227](https://github.com/mfontanini/presenterm/issues/227)).

## Fixes

* Avoid re-rendering code output and auto rendered blocks ([#280](https://github.com/mfontanini/presenterm/issues/280)).
* Use unicode width to calculate execution output's line len ([#261](https://github.com/mfontanini/presenterm/issues/261)).
* Display background color behind '\t' in code exec output ([#245](https://github.com/mfontanini/presenterm/issues/245)).
* Close child process stdin by default ([#297](https://github.com/mfontanini/presenterm/issues/297)).

## Improvements

* Update install instructions for Arch Linux ([#248](https://github.com/mfontanini/presenterm/issues/248)) - thanks @orhun.
* Fix all clippy warnings ([#231](https://github.com/mfontanini/presenterm/issues/231)) - thanks @mikavilpas.
* Include strict `_front_matter_parsing` in default config ([#229](https://github.com/mfontanini/presenterm/issues/229)) - thanks @mikavilpas.
* `CHANGELOG.md` contains clickable links to issues ([#230](https://github.com/mfontanini/presenterm/issues/230)) - thanks @mikavilpas.
* Add Support for Ruby Code Highlighting ([#226](https://github.com/mfontanini/presenterm/issues/226)) - thanks @pranavrao145.
* Use ".presenterm" as prefix for tmp files ([#306](https://github.com/mfontanini/presenterm/issues/306)).
* Add more descriptive error message when loading image fails ([#298](https://github.com/mfontanini/presenterm/issues/298)).
* Align all error messages to left ([#301](https://github.com/mfontanini/presenterm/issues/301)).

# v0.7.0 - 2024-03-02

## New features

* Add color to prefix in block quote ([#218](https://github.com/mfontanini/presenterm/issues/218)).
* Allow having code blocks without background ([#215](https://github.com/mfontanini/presenterm/issues/215) [#216](https://github.com/mfontanini/presenterm/issues/216)).
* Allow validating whether presentation overflows terminal ([#209](https://github.com/mfontanini/presenterm/issues/209) [#211](https://github.com/mfontanini/presenterm/issues/211)).
* Add parameter to list themes ([#207](https://github.com/mfontanini/presenterm/issues/207)).
* Add catppuccin themes ([#197](https://github.com/mfontanini/presenterm/issues/197) [#205](https://github.com/mfontanini/presenterm/issues/205) [#206](https://github.com/mfontanini/presenterm/issues/206)) - thanks @Mawdac.
* Detect konsole terminal emulator ([#204](https://github.com/mfontanini/presenterm/issues/204)).
* Allow customizing slide title style ([#201](https://github.com/mfontanini/presenterm/issues/201)).

## Fixes

* Don't crash in present mode ([#210](https://github.com/mfontanini/presenterm/issues/210)).
* Set colors properly before displaying an error ([#212](https://github.com/mfontanini/presenterm/issues/212)).

## Improvements

* Suggest a tool is missing when spawning returns ENOTFOUND ([#221](https://github.com/mfontanini/presenterm/issues/221)).
* Sort input file list ([#202](https://github.com/mfontanini/presenterm/issues/202)) - thanks @bmwiedemann.
* Add more example presentations ([#217](https://github.com/mfontanini/presenterm/issues/217)).
* Add Scoop to package managers ([#200](https://github.com/mfontanini/presenterm/issues/200)) - thanks @nagromc.
* Remove support for uncommon image formats ([#208](https://github.com/mfontanini/presenterm/issues/208)).

# v0.6.1 - 2024-02-11

## Fixes

* Don't escape symbols in block quotes ([#195](https://github.com/mfontanini/presenterm/issues/195)).
* Respect `XDG_CONFIG_HOME` when loading configuration files and custom themes ([#193](https://github.com/mfontanini/presenterm/issues/193)).

# v0.6.0 - 2024-02-09

## Breaking changes

* The default configuration file and custom themes paths have been changed in Windows and macOS to be compliant to where 
  those platforms store these types of files. See the [configuration 
  guide](https://mfontanini.github.io/presenterm/guides/configuration.html) to learn more.

## New features

* Add `f` keys, tab, and backspace as possible bindings ([#188](https://github.com/mfontanini/presenterm/issues/188)).
* Add support for multiline block quotes ([#184](https://github.com/mfontanini/presenterm/issues/184)).
* Use theme color as background on ascii-blocks mode images ([#182](https://github.com/mfontanini/presenterm/issues/182)).
* Blend ascii-blocks image semi-transparent borders ([#185](https://github.com/mfontanini/presenterm/issues/185)).
* Respect Windows/macOS config paths for configuration ([#181](https://github.com/mfontanini/presenterm/issues/181)).
* Allow making front matter strict parsing optional ([#190](https://github.com/mfontanini/presenterm/issues/190)).

## Fixes

* Don't add an extra line after an end slide shorthand ([#187](https://github.com/mfontanini/presenterm/issues/187)).
* Don't clear input state on key release event ([#183](https://github.com/mfontanini/presenterm/issues/183)).

# v0.5.0 - 2024-01-26

## New features

* Support images on Windows ([#120](https://github.com/mfontanini/presenterm/issues/120)).
* Support animated gifs on kitty terminal ([#157](https://github.com/mfontanini/presenterm/issues/157) [#161](https://github.com/mfontanini/presenterm/issues/161)).
* Support images on tmux running in kitty terminal ([#166](https://github.com/mfontanini/presenterm/issues/166)).
* Improve sixel support ([#169](https://github.com/mfontanini/presenterm/issues/169) [#172](https://github.com/mfontanini/presenterm/issues/172)).
* Use synchronized updates to remove flickering when switching slides ([#156](https://github.com/mfontanini/presenterm/issues/156)).
* Add newlines command ([#167](https://github.com/mfontanini/presenterm/issues/167)).
* Detect image protocol instead of relying on viuer ([#160](https://github.com/mfontanini/presenterm/issues/160)).
* Turn documentation into mdbook ([#141](https://github.com/mfontanini/presenterm/issues/141) [#147](https://github.com/mfontanini/presenterm/issues/147)) - thanks @pwnwriter.
* Allow using thematic breaks to end slides ([#138](https://github.com/mfontanini/presenterm/issues/138)).
* Allow specifying the preferred image protocol via `--image-protocol` / config file ([#136](https://github.com/mfontanini/presenterm/issues/136) [#170](https://github.com/mfontanini/presenterm/issues/170)).
* Add slide index modal ([#128](https://github.com/mfontanini/presenterm/issues/128) [#139](https://github.com/mfontanini/presenterm/issues/139) [#133](https://github.com/mfontanini/presenterm/issues/133) [#158](https://github.com/mfontanini/presenterm/issues/158)).
* Allow defining custom keybindings in config file ([#132](https://github.com/mfontanini/presenterm/issues/132) [#155](https://github.com/mfontanini/presenterm/issues/155)).
* Add key bindings modal ([#152](https://github.com/mfontanini/presenterm/issues/152)).
* Prioritize CLI args `--theme` over anything else ([#116](https://github.com/mfontanini/presenterm/issues/116)).
* Allow enabling automatic list pauses ([#106](https://github.com/mfontanini/presenterm/issues/106) [#109](https://github.com/mfontanini/presenterm/issues/109) [#110](https://github.com/mfontanini/presenterm/issues/110)).
* Allow passing in config file path via CLI arg ([#174](https://github.com/mfontanini/presenterm/issues/174)).

## Fixes

* Shrink columns layout dimensions correctly when shrinking left ([#113](https://github.com/mfontanini/presenterm/issues/113)).
* Explicitly set execution output foreground color in built-in themes ([#122](https://github.com/mfontanini/presenterm/issues/122)).
* Detect sixel early and fallback to ascii blocks properly ([#135](https://github.com/mfontanini/presenterm/issues/135)).
* Exit with a clap error on missing path ([#150](https://github.com/mfontanini/presenterm/issues/150)).
* Don't blow up if presentation file temporarily disappears ([#154](https://github.com/mfontanini/presenterm/issues/154)).
* Parse front matter properly in presence of \r\n ([#162](https://github.com/mfontanini/presenterm/issues/162)).
* Don't preload graphics mode when generating pdf metadata ([#168](https://github.com/mfontanini/presenterm/issues/168)).
* Ignore key release events ([#119](https://github.com/mfontanini/presenterm/issues/119)).

## Improvements

* Validate that config file contains the right attributes ([#107](https://github.com/mfontanini/presenterm/issues/107)).
* Display first presentation load error as any other ([#118](https://github.com/mfontanini/presenterm/issues/118)).
* Add hashes for windows artifacts ([#126](https://github.com/mfontanini/presenterm/issues/126)).
* Remove arch packaging files ([#111](https://github.com/mfontanini/presenterm/issues/111)).
* Lower CPU and memory usage when displaying images ([#157](https://github.com/mfontanini/presenterm/issues/157)).

# v0.4.1 - 2023-12-22

## New features

* Cause an error if an unknown field name is found on a theme, config file, or front matter ([#102](https://github.com/mfontanini/presenterm/issues/102)).

## Fixes

* Explicitly disable kitty/iterm protocols when printing images in export PDF mode as this was causing PDF generation in 
  macOS to fail ([#101](https://github.com/mfontanini/presenterm/issues/101)).

# v0.4.0 - 2023-12-16

## New features

* Add support for all of [bat](https://github.com/sharkdp/bat)'s code highlighting themes ([#67](https://github.com/mfontanini/presenterm/issues/67)).
* Add `terminal-dark` and `terminal-light` themes that preserve the terminal's colors and background ([#68](https://github.com/mfontanini/presenterm/issues/68) [#69](https://github.com/mfontanini/presenterm/issues/69)).
* Allow placing themes in `$HOME/.config/presenterm/themes` to make them available automatically as if they were 
  built-in themes ([#73](https://github.com/mfontanini/presenterm/issues/73)).
* Allow configuring the default theme in `$HOME/.config/presenterm/config.yaml` ([#74](https://github.com/mfontanini/presenterm/issues/74)).
* Add support for rendering _LaTeX_ and _typst_ code blocks automatically as images ([#75](https://github.com/mfontanini/presenterm/issues/75) [#76](https://github.com/mfontanini/presenterm/issues/76) [#79](https://github.com/mfontanini/presenterm/issues/79) [#81](https://github.com/mfontanini/presenterm/issues/81)).
* Add syntax highlighting support for _nix_ and _diff_ ([#78](https://github.com/mfontanini/presenterm/issues/78) [#82](https://github.com/mfontanini/presenterm/issues/82)).
* Add comment command to jump into the middle of a slide ([#86](https://github.com/mfontanini/presenterm/issues/86)).
* Add configuration option to have implicit slide ends ([#87](https://github.com/mfontanini/presenterm/issues/87) [#89](https://github.com/mfontanini/presenterm/issues/89)).
* Add configuration option to have custom comment-command prefix ([#91](https://github.com/mfontanini/presenterm/issues/91)).

# v0.3.0 - 2023-11-24

## New features

* Support more languages in code blocks thanks to [bat](https://github.com/sharkdp/bat)'s syntax sets ([#21](https://github.com/mfontanini/presenterm/issues/21) [#53](https://github.com/mfontanini/presenterm/issues/53)).
* Add shell script executable code blocks ([#17](https://github.com/mfontanini/presenterm/issues/17)).
* Allow exporting presentation to PDF ([#43](https://github.com/mfontanini/presenterm/issues/43) [#60](https://github.com/mfontanini/presenterm/issues/60)).
* Pauses no longer create new slides ([#18](https://github.com/mfontanini/presenterm/issues/18) [#25](https://github.com/mfontanini/presenterm/issues/25) [#34](https://github.com/mfontanini/presenterm/issues/34) [#42](https://github.com/mfontanini/presenterm/issues/42)).
* Allow display code block line numbers ([#46](https://github.com/mfontanini/presenterm/issues/46)).
* Allow code block selective line highlighting ([#48](https://github.com/mfontanini/presenterm/issues/48)).
* Allow code block dynamic line highlighting ([#49](https://github.com/mfontanini/presenterm/issues/49)).
* Support animated gifs when using the iterm2 image protocol ([#56](https://github.com/mfontanini/presenterm/issues/56)).
* Nix flake packaging ([#11](https://github.com/mfontanini/presenterm/issues/11) [#27](https://github.com/mfontanini/presenterm/issues/27)).
* Arch repo packaging ([#10](https://github.com/mfontanini/presenterm/issues/10)).
* Ignore vim-like code folding tags in comments.
* Add keybinding to refresh assets in presentation ([#38](https://github.com/mfontanini/presenterm/issues/38)).
* Template style footer is now one row above bottom ([#39](https://github.com/mfontanini/presenterm/issues/39)).
* Add `light` theme.

## Fixes

* Don't crash on Windows when terminal window size can't be found ([#14](https://github.com/mfontanini/presenterm/issues/14)).
* Don't reset numbers on ordered lists when using pauses in between ([#19](https://github.com/mfontanini/presenterm/issues/19)).
* Show proper line number when parsing a comment command fails ([#29](https://github.com/mfontanini/presenterm/issues/29) [#40](https://github.com/mfontanini/presenterm/issues/40)).
* Don't reset the default footer when overriding theme in presentation without setting footer ([#52](https://github.com/mfontanini/presenterm/issues/52)).
* Don't let code blocks/block quotes that don't fit on the screen cause images to overlap with text ([#57](https://github.com/mfontanini/presenterm/issues/57)).

# v0.2.1 - 2023-10-18

## New features

* Binary artifacts are now automatically generated when a new release is done ([#5](https://github.com/mfontanini/presenterm/issues/5)) - thanks @pwnwriter.

# v0.2.0 - 2023-10-17

## New features

* [Column layouts](https://github.com/mfontanini/presenterm/blob/26e2eb28884675aac452f4c6e03f98413654240c/docs/layouts.md) that let you structure slides into columns.
* Support for `percent` margin rather than only a fixed number of columns.
* Spacebar now moves the presentation into the next slide.
* Add support for `center` footer when using the `template` mode.
* **Breaking**: themes now only use colors in hex format.

## Fixes

* Allow using `sh` as language for code block ([#3](https://github.com/mfontanini/presenterm/issues/3)).
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
