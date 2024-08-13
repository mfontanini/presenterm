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

### Executing code blocks

Annotating a code block with a `+exec` attribute will make it executable. Once you're in a slide that contains an
executable block, press `control+e` to execute it. The output of the execution will be displayed on a box below the
code. The code execution is stateful so if you switch to another slide and then go back, you will still see the output.

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
* python
* ruby
* rust-script
* rust
* sh
* zsh

If there's a language that is not in this list and you would like it to be supported, please [create an 
issue](https://github.com/mfontanini/presenterm/issues/new) providing details on how to compile (if necessary) and run 
snippets for that language. You can also configure how to run code snippet for a language locally in your [config 
file](configuration.html#custom-snippet-executors).

[![asciicast](https://asciinema.org/a/BbAY817esxagCgPtnKUwgYnHr.svg)](https://asciinema.org/a/BbAY817esxagCgPtnKUwgYnHr)

> **Note**: because this is spawning a process and executing code, you should use this at your own risk.

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
is loaded. The languages that currently support this are _LaTeX_ and _typst_ where the contents of the code block is 
transformed into an image, allowing you to define formulas as text in your presentation. This can be done by using the 
`+render` attribute on a code block.

See the [LaTeX and typst](latex.html) and [mermaid](mermaid.html) docs for more information.
