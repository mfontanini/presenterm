## Code highlighting

Code highlighting is supported for the following languages:

* ada
* asp
* awk
* bash
* batchfile
* C
* cmake
* crontab
* C#
* clojure
* C++
* CSS
* D
* diff
* docker
* dotenv
* elixir
* elm
* erlang
* go
* haskell
* HTML
* java
* javascript
* json
* kotlin
* latex
* lua
* makefile
* markdown
* nix
* ocaml
* perl
* php
* protobuf
* puppet
* python
* R
* ruby
* rust
* scala
* shell
* sql
* swift
* svelte
* toml
* terraform
* typescript
* xml
* yaml
* vue
* zig

### Enabling line numbers

If you would like line numbers to be shown on the left of a code block use the `+line_numbers` switch after specifying
the language in a code block:

~~~markdown
```rust +line_numbers
   fn hello_world() {
       println!("Hello world");
   }
```
~~~

### Selective highlighting

By default, the entire code block will be syntax-highlighted. If instead you only wanted a subset of it to be
highlighted, you can use braces and a list of either individual lines, or line ranges that you'd want to highlight.

~~~markdown
```rust {1,3,5-7}
   fn potato() -> u32 {         // 1: highlighted
                                // 2: not highlighted
       println!("Hello world"); // 3: highlighted
       let mut q = 42;          // 4: not highlighted
       q = q * 1337;            // 5: highlighted
       q                        // 6: highlighted
   }                            // 7: highlighted
```
~~~

### Dynamic highlighting

Similar to the syntax used for selective highlighting, dynamic highlighting will change which lines of the code in a
code block are highlighted every time you move to the next/previous slide.

This is achieved by using the separator `|` to indicate what sections of the code will be highlighted at a given time.
You can also use `all` to highlight all lines for a particular frame.

~~~markdown
```rust {1,3|5-7}
   fn potato() -> u32 {

       println!("Hello world");
       let mut q = 42;
       q = q * 1337;
       q
   }
```
~~~

In this example, lines 1 and 3 will be highlighted initially. Then once you press a key to move to the next slide, lines
1 and 3 will no longer be highlighted and instead lines 5 through 7 will. This allows you to create more dynamic
presentations where you can display sections of the code to explain something specific about each of them.

See this real example of how this looks like.

[![asciicast](https://asciinema.org/a/iCf4f6how1Ux3H8GNzksFUczI.svg)](https://asciinema.org/a/iCf4f6how1Ux3H8GNzksFUczI)

### Including external code snippets

The `file` snippet type can be used to specify an external code snippet that will be included and highlighted as usual. 

~~~markdown
```file +exec +line_numbers
path: snippet.rs
language: rust
```
~~~

### Showing a snippet without a background

Using the `+no_background` flag will cause the snippet to have no background. This is useful when combining it with the 
`+exec_replace` flag described further down.

## Snippet execution

### Executing code blocks

Annotating a code block with a `+exec` attribute will make it executable. Pressing `control+e` when viewing a slide that 
contains an executable block, the code in the snippet will be executed and the output of the execution will be displayed 
on a box below it. The code execution is stateful so if you switch to another slide and then go back, you will still see 
the output.

~~~markdown
```bash +exec
echo hello world
```
~~~

Code execution **must be explicitly enabled** by using either:

* The `-x` command line parameter when running _presenterm_.
* Setting the `snippet.exec.enable` property to `true` in your [_presenterm_ config 
file](configuration.html#snippet-execution).

---

The list of languages that support execution are:

* bash
* c++
* c
* fish
* go
* java
* js
* kotlin
* lua
* nushell
* perl
* php
* python
* ruby
* rust
* rust-script: this highlights as normal Rust but uses [rust-script](https://rust-script.org/) to execute the snippet so 
it lets you use dependencies.
* sh
* zsh

If there's a language that is not in this list and you would like it to be supported, please [create an 
issue](https://github.com/mfontanini/presenterm/issues/new) providing details on how to compile (if necessary) and run 
snippets for that language. You can also configure how to run code snippet for a language locally in your [config 
file](configuration.html#custom-snippet-executors).

[![asciicast](https://asciinema.org/a/BbAY817esxagCgPtnKUwgYnHr.svg)](https://asciinema.org/a/BbAY817esxagCgPtnKUwgYnHr)

> **Note**: because this is spawning a process and executing code, you should use this at your own risk.

### Executing and replacing

Similar to `+exec`, `+exec_replace` causes a snippet to be executable but:

* Execution happens automatically without user intervention.
* The snippet will be automatically replaced with its execution output.

This can be useful to run programs that generate some form of ASCII art that you'd like to generate dynamically.

[![asciicast](https://asciinema.org/a/hklQARZKb5sP5mavL4cGgbYXD.svg)](https://asciinema.org/a/hklQARZKb5sP5mavL4cGgbYXD)

Because of the risk involved in `+exec_replace`, where code gets automatically executed when running a presentation, 
this requires users to explicitly opt in to it. This can be done by either passing in the `-X` command line parameter
or setting the `snippet.exec_replace.enable` flag in your configuration file to `true`. 

### Executing snippets that need a TTY

If you're trying to execute a program like `top` that needs to run on a TTY as it renders text, clears the screen, etc, 
you can use the `+acquire_terminal` modifier on a code already marked as executable with `+exec`. Executing snippets 
tagged with these two attributes will cause _presenterm_ to suspend execution, the snippet will be invoked giving it the 
raw terminal to do whatever it needs, and upon its completion _presenterm_ will resume its execution.

[![asciicast](https://asciinema.org/a/AHfuJorCNRR8ZEnfwQSDR5vPT.svg)](https://asciinema.org/a/AHfuJorCNRR8ZEnfwQSDR5vPT)

### Styled execution output

Snippets that generate output which contains escape codes that change the colors or styling of the text will be parsed 
and displayed respecting those styles. Do note that you may need to force certain tools to use colored output as they 
will likely not use it by default.

For example, to get colored output when invoking `ls` you can use:

~~~markdown
```bash +exec
ls /tmp --color=always
```
~~~

The parameter or way to enable this will depend on the tool being invoked.

### Hiding code lines

When you mark a code snippet as executable via the `+exec` flag, you may not be interested in showing _all the lines_ to 
your audience, as some of them may not be necessary to convey your point. For example, you may want to hide imports, 
non-essential functions, initialization of certain variables, etc. For this purpose, _presenterm_ supports a prefix 
under certain programming languages that let you indicate a line should be executed when running the code but should not 
be displayed in the presentation.

For example, in the following code snippet only the print statement will be displayed but the entire snippet will be 
ran:

~~~markdown
```rust
# fn main() {
println!("Hello world!");
# }
```
~~~

Rather than blindly relying on a prefix that may have a meaning in a language, prefixes are chosen on a per language 
basis. The languages that are supported and their prefix is:

* rust: `# `.
* python/bash/fish/shell/zsh/kotlin/java/javascript/typescript/c/c++/go: `/// `.

This means that any line in a rust code snippet that starts with `# ` will be hidden, whereas all lines in, say, a 
golang code snippet that starts with a `/// ` will be hidden.

### Pre-rendering 

Some languages support pre-rendering. This means the code block is transformed into something else when the presentation 
is loaded. The languages that currently support this are _mermaid_, _LaTeX_, and _typst_ where the contents of the code 
block is transformed into an image, allowing you to define formulas as text in your presentation. This can be done by 
using the `+render` attribute on a code block.

See the [LaTeX and typst](latex.html) and [mermaid](mermaid.html) docs for more information.

### Adding highlighting syntaxes for new languages

_presenterm_ uses the syntaxes supported by [bat](https://github.com/sharkdp/bat) to highlight code snippets, so any 
languages supported by _bat_ natively can be added to _presenterm_ easily. Please create a ticket or use 
[this](https://github.com/mfontanini/presenterm/pull/385) as a reference to submit a pull request to make a syntax 
officially supported by _presenterm_ as well.

If a language isn't natively supported by _bat_ but you'd like to use it, you can follow
[this guide in the bat docs](https://github.com/sharkdp/bat#adding-new-syntaxes--language-definitions) and
invoke _bat_ directly in a presentation:

~~~markdown
```bash +exec_replace
bat --color always script.py
```
~~~
