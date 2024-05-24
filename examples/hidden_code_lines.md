---
title: Hidden Code Lines
---

You can use the delimiter `"/// "` at the start of lines of code
that will not be visible in the code snippet in the presentation
(but will still be executed if it is `bash` code).

Execute the following `bash` code:

```bash +exec +line_numbers
/// echo "This echo was hidden, but still executes."
sleep 1
echo "This line is visible and executes as normal."
```

<!-- end_slide -->

# Line Numbering

This is how the line numbers render without any hidden code lines:

```rust +line_numbers
let foo = 2;
let bar = 2;
let sum = foo + bar;
println!("The sum is: {}", sum);
```

Observe that the line numbering in the following snippet matches the above.

The following snippet has a hidden enclosing main function, and a hidden code line between the visible lines 2 and 3:

```rust +line_numbers
/// fn main() {
let foo = 2;
let bar = 2;
/// This is also hidden, to show it works with interleaved hidden lines.
let sum = foo + bar;
println!("The sum is: {}", sum);
/// }
```

<!-- end_slide -->

# Selective Highlighting

The following snippet has a hidden enclosing main function, and a hidden code line between the visible lines 6 and 7.

The lines `{1,3,4,7,9-11}` should be highlighted, which match the visible code line numbers:

```rust {1,3,4,7,9-11} +line_numbers
/// fn main() {
println!("Hello world");
let mut q = 42;
q = q * 1337;
q = q * 1337;
q = q * 1337;
let foo = "Hidden line comes next";
/// This is also hidden, to show it works with interleaved hidden lines.
let bar = "Hidden line above";
q = q * 1337;
q = q * 1337;
q = q * 1337;
q = q * 1337;
/// }
```

<!-- end_slide -->

# Dynamic Highlighting

Dynamic highlighting also respects the hidden/visible code lines.

The following snippet has a hidden enclosing main function, and a hidden code line between the visible lines 3 and 4.

Here we highlight each line on each consectuive slide, in turn:

```rust {1|2|3|4} +line_numbers
/// fn main() {
println!("Hello world");
let mut q = 42;
let foo = "Hidden line comes next";
/// This is also hidden, to show it works with interleaved hidden lines.
let bar = "Hidden line above";
/// }
```
