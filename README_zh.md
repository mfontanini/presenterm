> **语言**：本文档为简体中文翻译，[点此查看英文原版 README.md](README.md)。

presenterm
===

[![crates-badge]][crates-package] [![brew-badge]][brew-package] [![nix-badge]][nix-package] 
[![arch-badge]][arch-package] [![scoop-badge]][scoop-package] [![winget-badge]][winget-package]

[brew-badge]: https://img.shields.io/homebrew/v/presenterm
[brew-package]: https://formulae.brew.sh/formula/presenterm
[nix-badge]: https://img.shields.io/badge/Packaged_for-Nix-5277C3.svg?logo=nixos&labelColor=73C3D5
[nix-package]: https://search.nixos.org/packages?size=1&show=presenterm
[crates-badge]: https://img.shields.io/crates/v/presenterm
[crates-package]: https://crates.io/crates/presenterm
[arch-badge]: https://img.shields.io/archlinux/v/extra/x86_64/presenterm
[arch-package]: https://archlinux.org/packages/extra/x86_64/presenterm/
[scoop-badge]: https://img.shields.io/scoop/v/presenterm
[scoop-package]: https://scoop.sh/#/apps?q=presenterm&id=a462290f824b50f180afbaa6d8c7c1e6e0952e3a
[winget-badge]: https://img.shields.io/winget/v/mfontanini.presenterm
[winget-package]: https://winstall.app/apps/mfontanini.presenterm

_presenterm_ 让你能够用 markdown 格式创建演示文稿，并在终端中运行它们，支持图片和动图、高度可定制的主题、代码高亮、将演示文稿导出为 PDF 格式，以及众多其他功能。这就是 [demo 演示文稿](/examples/demo.md) 在 [kitty 终端](https://sw.kovidgoyal.net/kitty/) 中运行时的样子：

![](/docs/src/assets/demo.gif)

在 [examples 目录](/examples) 中查看其余的示例演示文稿。

# 文档

请访问[文档][docs-introduction]入门。

# 特性

* 演示文稿由一[个或多个][docs-include] markdown 文件组成。
* 在 _kitty_、_iterm2_、_wezterm_、_ghostty_ 和 _foot_ 等终端上显示[图片与动图][docs-images]。
* [可定制的主题][docs-themes]，涵盖颜色、边距、布局（左对齐/居中内容）、每一页幻灯片的页脚等。多个[内置主题][docs-builtin-themes]无需你自行定义就能让你的演示文稿拥有你想要的观感。
* 支持[种类繁多编程语言][docs-code-highlight]的代码高亮。
* 为支持它的终端提供[字体大小][docs-font-sizes]。
* [选择性/动态][docs-selective-highlight]代码高亮，可一次只高亮部分代码。
* [列布局][docs-layout]。
* [mermaid 图渲染][docs-mermaid]。
* [d2 图渲染][docs-d2]。
* [_LaTeX_ 与 _typst_ 公式渲染][docs-latex]。
* [引言幻灯片][docs-intro-slide]，用于展示演示文稿标题和你的名字。
* [幻灯片标题][docs-slide-titles]。
* 支持多种编程语言的[代码片段执行][docs-code-execute]，包括在伪终端中执行。
* [将演示文稿导出为 PDF 和 HTML][docs-exports]。
* [幻灯片过渡效果][docs-slide-transitions]。
* [暂停][docs-pauses]你幻灯片的部分内容。
* [自定义键位绑定][docs-key-bindings]。
* [每次改动时自动重新加载你的演示文稿][docs-hot-reload]，获得快速的开发循环。
* [定义演讲者备注][docs-speaker-notes]，在演示过程中为你提供帮助。

想了解更多信息，请参阅[介绍页面][docs-introduction]。

# presenterm 实战

以下是一些使用 _presenterm_ 的演讲与演示：

- [If it can compute, it can run Ratatui][it-can-run-ratatui] by [Orhun Parmaksız][orhun-github]
- [Building Embedded TUIs with Rust & Ratatui][building-embedded-tuis] by [Orhun Parmaksız][orhun-github]
- [Bringing Terminal Aesthetics to the Web With Rust][bringing-terminal-aesthetics] by [Orhun Parmaksız][orhun-github]
- [7 Rust Terminal Tools That You Should Use][rust-terminal-tools] by [Orhun Parmaksız][orhun-github]
- [Renaissance of Terminal User Interfaces with Rust][renaissance-tui] by [Orhun Parmaksız][orhun-github]
- [Using Nix on Apple Silicon and declarative development environments][NiXOS-and-Dev] by [pwnwriter][pwnwriter-github]
- [Hayasen: A Robust Embedded Rust Library which supports multiple sensors][hayasen] by [Vaishnav-Sabari-Girish][vaishnav]
- [Using ratatui in Embedded sytems : Meet Mousefood][mousefood] by [Vaishnav-Sabari-Girish][vaishnav]

用 _presenterm_ 做过演讲？我们很乐意在此展示它！开一个 PR 或 issue 就能把它加进来。

<!-- links -->

[docs-introduction]: https://mfontanini.github.io/presenterm/
[docs-basics]: https://mfontanini.github.io/presenterm/features/introduction.html
[docs-intro-slide]: https://mfontanini.github.io/presenterm/features/introduction.html#introduction-slide
[docs-slide-titles]: https://mfontanini.github.io/presenterm/features/introduction.html#slide-titles
[docs-font-sizes]: https://mfontanini.github.io/presenterm/features/introduction.html#font-sizes
[docs-pauses]: https://mfontanini.github.io/presenterm/features/commands.html#pauses
[docs-images]: https://mfontanini.github.io/presenterm/features/images.html
[docs-include]: https://mfontanini.github.io/presenterm/features/commands.html#including-external-markdown-files
[docs-themes]: https://mfontanini.github.io/presenterm/features/themes/introduction.html
[docs-builtin-themes]: https://mfontanini.github.io/presenterm/features/themes/introduction.html#built-in-themes
[docs-code-highlight]: https://mfontanini.github.io/presenterm/features/code/highlighting.html
[docs-code-execute]: https://mfontanini.github.io/presenterm/features/code/execution.html
[docs-selective-highlight]: https://mfontanini.github.io/presenterm/features/code/highlighting.html#selective-highlighting
[docs-slide-transitions]: https://mfontanini.github.io/presenterm/features/slide-transitions.html
[docs-layout]: https://mfontanini.github.io/presenterm/features/layout.html
[docs-mermaid]: https://mfontanini.github.io/presenterm/features/code/mermaid.html
[docs-d2]: https://mfontanini.github.io/presenterm/features/code/d2.html
[docs-latex]: https://mfontanini.github.io/presenterm/features/code/latex.html
[docs-exports]: https://mfontanini.github.io/presenterm/features/exports.html
[docs-key-bindings]: https://mfontanini.github.io/presenterm/configuration/settings.html#key-bindings
[docs-hot-reload]: https://mfontanini.github.io/presenterm/features/introduction.html#hot-reload
[docs-speaker-notes]: https://mfontanini.github.io/presenterm/features/speaker-notes.html
[bat]: https://github.com/sharkdp/bat
[syntect]: https://github.com/trishume/syntect
[bringing-terminal-aesthetics]: https://www.youtube.com/watch?v=iepbyYrF_YQ
[building-embedded-tuis]: https://www.youtube.com/watch?v=F04kQMKwrwQ
[it-can-run-ratatui]: https://www.youtube.com/watch?v=btqNDDuZ3cI
[rust-terminal-tools]: https://www.youtube.com/watch?v=ATiKwUiqnAU
[renaissance-tui]: https://www.youtube.com/watch?v=hWG51Mc1DlM
[orhun-github]: https://github.com/orhun
[NiXOS-and-Dev]: https://github.com/pwnwriter/PTN11
[pwnwriter-github]: https://github.com/pwnwriter
[hayasen]: https://github.com/Vaishnav-Sabari-Girish/rust_bangalore_oct_2025
[vaishnav]: https://github.com/Vaishnav-Sabari-Girish
[mousefood]: https://github.com/Vaishnav-Sabari-Girish/rust_bangalore_december_2025/
