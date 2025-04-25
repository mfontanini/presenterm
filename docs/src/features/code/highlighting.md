# Code highlighting

Code highlighting is supported for the following languages:

| Language   | Execution support |
| -----------|-------------------|
| ada        |                   |
| asp        |                   |
| awk        |                   |
| bash       |         ✓         |
| batchfile  |                   |
| C          |         ✓         |
| cmake      |                   |
| crontab    |                   |
| C#         |         ✓         |
| clojure    |                   |
| C++        |         ✓         |
| CSS        |                   |
| D          |                   |
| diff       |                   |
| docker     |                   |
| dotenv     |                   |
| elixir     |                   |
| elm        |                   |
| erlang     |                   |
| fish       |         ✓         |
| go         |         ✓         |
| haskell    |         ✓         |
| HTML       |                   |
| java       |         ✓         |
| javascript |         ✓         |
| json       |                   |
| julia      |         ✓         |
| kotlin     |         ✓         |
| latex      |                   |
| lua        |         ✓         |
| makefile   |                   |
| markdown   |                   |
| nix        |                   |
| ocaml      |                   |
| perl       |         ✓         |
| php        |         ✓         |
| protobuf   |                   |
| puppet     |                   |
| python     |         ✓         |
| R          |         ✓         |
| ruby       |         ✓         |
| rust       |         ✓         |
| scala      |                   |
| shell      |         ✓         |
| sql        |                   |
| swift      |                   |
| svelte     |                   |
| tcl        |                   |
| toml       |                   |
| terraform  |                   |
| typescript |                   |
| xml        |                   |
| yaml       |                   |
| vue        |                   |
| zig        |                   |
| zsh        |         ✓         |

Other languages that are supported are:

* nushell, for which highlighting isn't supported but execution is.
* rust-script, which is highlighted as rust but is executed via the [rust-script](https://rust-script.org/) tool,
which lets you specify dependencies in your snippet.

If there's a language that is not in this list and you would like it to be supported, please [create an 
issue](https://github.com/mfontanini/presenterm/issues/new). If you'd also like code execution support, provide details 
on how to compile (if necessary) and run snippets for that language. You can also configure how to run code snippet for 
a language locally in your [config file](../../configuration/settings.md#custom-snippet-executors).

## Enabling line numbers

If you would like line numbers to be shown on the left of a code block use the `+line_numbers` switch after specifying
the language in a code block:

~~~markdown
```rust +line_numbers
   fn hello_world() {
       println!("Hello world");
   }
```
~~~

## Selective highlighting

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

## Dynamic highlighting

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

## Including external code snippets

The `file` snippet type can be used to specify an external code snippet that will be included and highlighted as usual. 

~~~markdown
```file +exec +line_numbers
path: snippet.rs
language: rust
```
~~~

If you'd like to include only a subset of the file, you can use the optional fields `start_line` and `end_line`:

~~~markdown
```file +exec +line_numbers
path: snippet.rs
language: rust
# Only shot lines 5-10
start_line: 5
end_line: 10
```
~~~

## Showing a snippet without a background

Using the `+no_background` flag will cause the snippet to have no background. This is useful when combining it with the 
`+exec_replace` flag described further down.

## Adding highlighting syntaxes for new languages

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

> [!note]
> Check the [code execution docs](execution.md#executing-and-replacing) for more details on how to allow the tool to run 
> `exec_replace` blocks.
