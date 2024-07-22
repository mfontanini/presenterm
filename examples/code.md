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

Snippet execution
===

Run code snippets from the presentation and display their output dynamically.

```python +exec
/// import time
for i in range(0, 5):
    print(f"count is {i}")
    time.sleep(0.5)
```

<!-- end_slide -->

Snippet execution - `stderr`
===

Output from `stderr` will also be shown as output.

```bash +exec
echo "This is a successful command"
sleep 0.5
echo "This message redirects to stderr" >&2
sleep 0.5
echo "This is a successful command again"
sleep 0.5
man # Missing argument
```
