---
title: Introducing _presenterm_
author: Matias
---

Customizability
---

_presenterm_ allows configuring almost anything about your presentation:

* The colors used.
* Layouts.
* Footers, including images in the footer.

<!-- pause -->

This is an example on how to configure a footer:

```yaml
footer:
  style: template
  left:
    image: doge.png
  center: '<span class="noice">Colored</span> _footer_'
  right: "{current_slide} / {total_slides}"
  height: 5

palette:
  classes:
    noice:
      foreground: red
```

<!-- end_slide -->

Headers
---

Markdown headers can be used to set slide titles like:

```markdown
Headers
-------
```

# Headers

Each header type can be styled differently.

## Subheaders

### And more

<!-- end_slide -->

Code highlighting
---

Highlight code in 50+ programming languages:

```rust
// Rust
fn greet() -> &'static str {
    "hi mom"
}
```

```python
# Python
def greet() -> str:
    return "hi mom"
```

<!-- pause -->

-------

Code snippets can have different styles including no background:

```cpp +no_background +line_numbers
// C++
string greet() {
    return "hi mom";
}
```

<!-- end_slide -->

Dynamic code highlighting
---

Dynamically highlight different subsets of lines:

```rust {1-4|6-10|all} +line_numbers
#[derive(Clone, Debug)]
struct Person {
    name: String,
}

impl Person {
    fn say_hello(&self) {
        println!("hello, I'm {}", self.name)
    }
}
```

<!-- end_slide -->

Snippet execution
---

Code snippets can be executed on demand:

* For 20+ languages, including compiled ones.
* Display their output in real time.
* Comment out unimportant lines to hide them.

```rust +exec
# use std::thread::sleep;
# use std::time::Duration;
fn main() {
    let names = ["Alice", "Bob", "Eve", "Mallory", "Trent"];
    for name in names {
        println!("Hi {name}!");
        sleep(Duration::from_millis(500));
    }
}
```

<!-- end_slide -->

Images
---

Images and animated gifs are supported in terminals such as:

* kitty
* iterm2
* wezterm
* ghostty
* Any sixel enabled terminal

<!-- column_layout: [1, 3, 1] -->

<!-- column: 1 -->

![](doge.png)

_Picture by Alexis Bailey / CC BY-NC 4.0_

<!-- end_slide -->

Column layouts
---

<!-- column_layout: [7, 3] -->

<!-- column: 0 -->

Use column layouts to structure your presentation:

* Define the number of columns.
* Adjust column widths as needed.
* Write content into every column.

```rust
fn potato() -> u32 {
    42
}
```

<!-- column: 1 -->

![](doge.png)

<!-- reset_layout -->

---

Layouts can be reset at any time.

```python
print("Hello world!")
```

<!-- end_slide -->

Text formatting
---

Text formatting works including:

* **Bold text**.
* _Italics_.
* **_Bold and italic_**.
* ~Strikethrough~.
* `Inline code`.
* Links [](https://example.com/)
* <span style="color: red">Colored</span> text.
* <span style="color: blue; background-color: black">Background color</span> can be changed too.

<!-- end_slide -->

More markdown
---

Other markdown elements supported are:

# Block quotes

> Lorem ipsum dolor sit amet. Eos laudantium animi ut ipsam beataeet
> et exercitationem deleniti et quia maiores a cumque enim et
> aspernatur nesciunt sed adipisci quis.

# Alerts

> [!caution]
> Github style alerts

# Tables

| Name   | Taste  |
| ------ | ------ |
| Potato | Great  |
| Carrot | Yuck   |

<!-- end_slide -->

<!-- jump_to_middle -->

The end
---
