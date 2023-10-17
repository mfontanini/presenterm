---
title: Introducing presenterm
author: Matias
---

Introduction slide
---

An introduction slide can be defined by using a front matter at the beginning of the markdown file:

```yaml
---
title: My presentation title
sub_title: An optional subtitle
author: Your name which will appear somewhere in the bottom
---
```

The slide's theme can also be configured in the front matter:

```yaml
---
theme:
  # Specify it by name for built-in themes
  name: my-favorite-theme

  # Otherwise specify the path for it
  path: /home/myself/themes/epic.yaml

  # Or override parts of the theme right here
  override:
    default:
      colors:
        foreground: white
---
```

<!-- end_slide -->

Headers
---

Using commonmark setext headers allows you to set titles for your slides (like seen above!):

```
Headers
---
```

# Other headers

All other header types are simply treated as headers within your slide.

## Subheaders
### And more

<!-- end_slide -->

Slide commands
---

Certain commands in the form of HTML comments can be used:

# Ending slides

In order to end a single slide, use:

```html
<!-- end_slide -->
```

# Creating pauses

Slides can be paused by using the `pause` command:

```html
<!-- pause -->
```

This allows you to:

<!-- pause -->
* Create suspense.
<!-- pause -->
* Have more interactive presentations.
<!-- pause -->
* Possibly more!

<!-- end_slide -->

Code highlighting
---

Code highlighting is enabled for code blocks that include the most commonly used programming languages:

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

```cpp
// C++
string greet() {
    return "hi mom";
}
```

And many more!

<!-- end_slide -->

Text formatting
---

Text formatting works as expected:

* **This is bold text**.
* _This is italics_.
* **This is bold _and this is bold and italic_**.
* ~This is strikethrough text.~
* Inline code `is also supported`.
* Links look like this [](https://example.com/)

<!-- end_slide -->

Images
---

Image rendering is supported as long as you're using iterm2, your terminal supports
the kitty graphics protocol (such as the kitty terminal itself!), or the sixel format.

* Include images in your slides by using `![](path-to-image.extension)`.
* Images will be rendered in **their original size**.
    * If they're too big they will be scaled down to fit the screen.

![](doge.png)

_Picture by Alexis Bailey / CC BY-NC 4.0_

<!-- end_slide -->

Column layouts
---

<!-- column_layout: [2, 1] -->

<!-- column: 0 -->

Column layouts let you organize content into columns.

Here you can place code:

```rust
fn potato() -> u32 {
    42
}
```

Plus pretty much anything else:
* Bullet points.
* Images.
* _more_!

<!-- column: 1 -->

![](doge.png)

_Picture by Alexis Bailey / CC BY-NC 4.0_

<!-- reset_layout -->

Because we just reset the layout, this text is now below both of the columns. Code and any other element will now look 
like it usually does:

```python
print("Hello world!")
```

<!-- end_slide -->

Other elements
---

Other elements supported are:

# Tables

| Name | Taste |
| ------ | ------ |
| Potato | Great |
| Carrot | Yuck |

# Block quotes

> Lorem ipsum dolor sit amet. Eos laudantium animi ut ipsam beataeet
> et exercitationem deleniti et quia maiores a cumque enim et
> aspernatur nesciunt sed adipisci quis.

# Thematic breaks

A horizontal line by using `---`.

---

