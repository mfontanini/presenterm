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
