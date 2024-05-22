---
theme:
  override:
    code:
      alignment: left
      background: false
---

Code styling
===

This presentation shows how to:

* Left-align code blocks.
* Have code blocks without background.
* Execute code snippets.

```rust
pub struct Greeter {
    prefix: &'static str,
}

impl Greeter {
    /// Greet someone.
    pub fn greet(&self, name: &str) -> String {
        let prefix = self.prefix;
        format!("{prefix} {name}!")
    }
}

fn main() {
    let greeter = Greeter { prefix: "Oh, hi" };
    let greeting = greeter.greet("Mark");
    println!("{greeting}");
}
```

<!-- end_slide -->

Column layouts
===

The same code as the one before but split into two columns to split the API definition with its usage:

<!-- column_layout: [1, 1] -->

<!-- column: 0 -->

# The `Greeter` type

```rust
pub struct Greeter {
    prefix: &'static str,
}

impl Greeter {
    /// Greet someone.
    pub fn greet(&self, name: &str) -> String {
        let prefix = self.prefix;
        format!("{prefix} {name}!")
    }
}
```

<!-- column: 1 -->

# Using the `Greeter`

```rust
fn main() {
    let greeter = Greeter {
      prefix: "Oh, hi"
    };
    let greeting = greeter.greet("Mark");
    println!("{greeting}");
}
```

<!-- end_slide -->

Code execution
===

Run commands from the presentation and display their output dynamically.

```bash +exec
for i in $(seq 1 5)
do
    echo "hi $i"
    sleep 0.5
done
```

<!-- end_slide -->

Code execution - `stderr`
===

Output from `stderr` will also be shown as output.

```bash +exec
echo "This is a successful command"
echo "This message redirects to stderr" >&2
echo "This is a successful command again"
man # Missing argument
```