# Snippet execution

## Executing code blocks

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
file](../../configuration/settings.md#snippet-execution).

Refer to [the table in the highlighting page](highlighting.md#code-highlighting) for the list of languages for which 
code execution is supported.

---

[![asciicast](https://asciinema.org/a/BbAY817esxagCgPtnKUwgYnHr.svg)](https://asciinema.org/a/BbAY817esxagCgPtnKUwgYnHr)

> [!warning]
> Run code in presentations at your own risk! Especially if you're running someone else's presentation. Don't blindly 
> enable snippet execution!

### Output placing

By default a snippet's output will always show up right below the snippet. However, if you wanted to show the output in 
a different place in a slide (e.g. another column) or even in another slide you can do this by:

1. Defining a snippet's identifier:

~~~markdown
```bash +exec +id:foo
echo hellow world
```
~~~

2. Referencing that identifier where you want the output to appear by using the `snippet_output` comment command:

~~~markdown
<!-- snippet_output: foo -->
~~~

A single snippet can be referenced multiple times in multiple slides, as long as the slide you're referencing it in 
comes after the snippet. The snippet will only be executed once, and every `snippet_output` command will display that 
single execution's output.

### Validating snippets

While you're developing your presentation you probably want to make sure the executable snippets you write in it are 
correct and don't contain any syntax errors. While you can do this by constantly pressing `<c-e>` every time you change 
a snippet, this is automatically done by _presenterm_ if you pass in the `--validate-snippets` flag.

When you pass in this flag, _presenterm_ will:

* Automatically run all `+exec`, `+exec_replace`, and `+validate` snippets as soon as your presentation starts. Note 
that the `+validate` flag is a special one that doesn't make a snippet executable but still validates it by running it 
during development.
* Report an error if any of the snippets returns an exit code other than 0.
* Re-run all snippets `+exec` and `+exec_replace` snippets every time the presentation is reloaded.

In case you expect a snippet to return an exit code other than 0, you can use the `+expect:failure` flag. This will 
cause _presenterm_ to display an error if the snippet does not fail.

For example, the following defines a snippet that's not executable but that will be validated if `--validate-snippets` 
is passed in and will display an error if the snippet does not fail.

```rust +validate +expect:failure
fn main() {
    let q = 42;
    let w = q + "foo"; // oops
}
```

## Executing and replacing

Similar to `+exec`, `+exec_replace` causes a snippet to be executable but:

* Execution happens automatically without user intervention.
* The snippet will be automatically replaced with its execution output.

This can be useful to run programs that generate some form of ASCII art that you'd like to generate dynamically.

[![asciicast](https://asciinema.org/a/hklQARZKb5sP5mavL4cGgbYXD.svg)](https://asciinema.org/a/hklQARZKb5sP5mavL4cGgbYXD)

Because of the risk involved in `+exec_replace`, where code gets automatically executed when running a presentation, 
this requires users to explicitly opt in to it. This can be done by either passing in the `-X` command line parameter
or setting the `snippet.exec_replace.enable` flag in your configuration file to `true`.

## Alternative executors

Some languages support alternative executors. For example, `rust` code can be ran via 
[`rust-script`](https://rust-script.org/), which allows you to use external crates. These executors can be used by 
specifying `:<executor-name>` after `+exec` or `+exec_replace`.

For example, the following `rust` snippet will be executed using `rust-script`:

~~~markdown
```rust +exec:rust-script
# //! ```cargo
# //! [dependencies]
# //! time = "0.1.25"
# //! ```
# // The lines above will be hidden
fn main() {
    println!("the time is {}", time::now().rfc822z());
}
```
~~~

The supported alternative executors are:

* `rust-script` for `rust` snippets.
* `pytest` and `uv` for `python` snippets.

## Code to image conversions

The `+image` attribute behaves like `+exec_replace` but also assumes the output of the executed snippet will be an 
image, and it will render it as such. For this to work, the code **must only emit an image in jpg/png formats** and 
nothing else.

For example, this would render the demo presentation's image:

~~~markdown
```bash +image
cat examples/doge.png
```
~~~

This attribute carries the same risks as `+exec_replace` and therefore needs to be enabled via the same flags.

## Executing snippets that need a TTY

If you're trying to execute a program like `top` that needs to run on a TTY as it renders text, clears the screen, etc, 
you can use the `+acquire_terminal` modifier on a code already marked as executable with `+exec`. Executing snippets 
tagged with these two attributes will cause _presenterm_ to suspend execution, the snippet will be invoked giving it the 
raw terminal to do whatever it needs, and upon its completion _presenterm_ will resume its execution.

[![asciicast](https://asciinema.org/a/AHfuJorCNRR8ZEnfwQSDR5vPT.svg)](https://asciinema.org/a/AHfuJorCNRR8ZEnfwQSDR5vPT)

## Styled execution output

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

## Hiding code lines

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

## Pre-rendering 

Some languages support pre-rendering. This means the code block is transformed into something else when the presentation 
is loaded. The languages that currently support this are _mermaid_, _LaTeX_, and _typst_ where the contents of the code 
block is transformed into an image, allowing you to define formulas as text in your presentation. This can be done by 
using the `+render` attribute on a code block.

See the [LaTeX and typst](latex.md), [mermaid](mermaid.md), and [d2](d2.md) docs for more information.

