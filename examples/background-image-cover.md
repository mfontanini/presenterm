---
title: "fit: cover"
sub_title: Background image — cover mode
author: presenterm
theme:
  override:
    default:
      colors:
        foreground: "e6e6e6"
        background: "040312"
      background_image:
        path: bg-square.png
        fit: cover
---

cover — square image (800x800)
---

The image is scaled to **cover** the full terminal while keeping its
aspect ratio. Part of the image may be clipped.

Circles should look **round** (not distorted).

<!-- end_slide -->

Details
---

* The square image scales up until it covers the full terminal
* Edges are cropped evenly from all sides
* Try resizing the terminal — the image re-renders to always cover
* The background **color** shows through any uncovered area
