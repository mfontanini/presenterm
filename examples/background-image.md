---
title: "fit: stretch"
sub_title: Background image — stretch mode (default)
author: presenterm
theme:
  override:
    default:
      colors:
        foreground: "e6e6e6"
        background: "040312"
      background_image:
        path: bg-wide.png
        fit: stretch
---

stretch — wide image (1920x1080)
---

The image is **stretched** to fill the exact terminal dimensions,
ignoring aspect ratio.

Circles may appear slightly **distorted** because the image is forced
to match the exact terminal dimensions.

<!-- end_slide -->

Details
---

* The image fills every cell of the terminal — no gaps, no clipping
* Aspect ratio is not preserved, so shapes may be distorted
* This is the simplest mode and the default when `fit` is omitted
* Try resizing — the distortion changes as the terminal shape changes
