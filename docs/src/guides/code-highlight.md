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

~~~
```rust +line_numbers
   fn hello_world() {
       println!("Hello world");
   }
```
~~~

### Selective highlighting

By default, the entire code block will be syntax-highlighted. If instead you only wanted a subset of it to be
highlighted, you can use braces and a list of either individual lines, or line ranges that you'd want to highlight.

~~~
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

~~~
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

[![asciicast](https://asciinema.org/a/dpXDXJoJRRX4mQ7V6LdR3rO2z.svg)](https://asciinema.org/a/dpXDXJoJRRX4mQ7V6LdR3rO2z)

### Executing code

Annotating a shell code block with a `+exec` switch will make it executable. Once you're in a slide that contains an
executable block, press `control+e` to execute it. The output of the execution will be displayed on a box below the
code. The code execution is stateful so if you switch to another slide and then go back, you will still see the output.

~~~
```bash +exec
echo hello world
```
~~~

Note that using `bash`, `zsh`, `fish`, etc, will end up using that specific shell to execute your script.

[![asciicast](https://asciinema.org/a/gnzjXpVSOwOiyUqQvhi0AaHG7.svg)](https://asciinema.org/a/gnzjXpVSOwOiyUqQvhi0AaHG7)

> **Note**: because this is spawning a process and executing code, you should use this at your own risk.

### Pre-rendering 

Some languages support pre-rendering. This means the code block is transformed into something else when the presentation 
is loaded. The languages that currently support this are _LaTeX_ and _typst_ where the contents of the code block is 
transformed into an image, allowing you to define formulas as text in your presentation. This can be done by using the 
`+render` attribute on a code block.

See the [LaTeX and typst docs](latex.html) for more information.
