---
title: "fit: contain"
sub_title: Background image — contain mode
author: presenterm
theme:
  override:
    default:
      colors:
        foreground: "e6e6e6"
        background: "040312"
      background_image:
        path: bg-tall.png
        opacity: 80
        fit: contain
---

contain — tall image (540x960) at 80% opacity
---

The image is scaled to **fit entirely** within the terminal while keeping
its aspect ratio. The background color fills the remaining space.

Circles should look **round** (not distorted). The full image is visible.

<!-- end_slide -->

Details
---

* The tall image fits within the terminal height
* Dark bars (the background color "040312") appear on the sides
* The image is centered both horizontally and vertically
* Try resizing — the image re-centers and rescales to always fit
