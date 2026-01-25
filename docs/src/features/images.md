# Images

Images are supported and will render in your terminal as long as it supports either the [iterm2 image 
protocol](https://iterm2.com/documentation-images.html), the [kitty graphics 
protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/), or [sixel](https://saitoha.github.io/libsixel/). Some of 
the terminals where at least one of these is supported are:

* [kitty](https://sw.kovidgoyal.net/kitty/)
* [iterm2](https://iterm2.com/)
* [WezTerm](https://wezfurlong.org/wezterm/index.html)
* [ghostty](https://ghostty.org/)
* [foot](https://codeberg.org/dnkl/foot)

---

Things you should know when using image tags in your presentation's markdown are:
* Image paths are relative to your presentation path. That is a tag like `![](food/potato.png)` will be looked up at 
  `$PRESENTATION_DIRECTORY/food/potato.png`.
* Images will be rendered by default in their original size. That is, if your terminal is 300x200px and your image is 
200x100px, it will take up 66% of your horizontal space and 50% of your vertical space.
* The exception to the point above is if the image does not fit in your terminal, it will be resized accordingly while 
  preserving the aspect ratio.
* If your terminal does not support any of the graphics protocol above, images will be rendered using ascii blocks. It 
  ain't great but it's something!
* Remote images are not supported [by design](https://github.com/mfontanini/presenterm/issues/213#issuecomment-1950342423).

## tmux

If you're using tmux, you will need to enable the [allow-passthrough 
option](https://github.com/tmux/tmux/wiki/FAQ#what-is-the-passthrough-escape-sequence-and-how-do-i-use-it) for images to 
work correctly.

## Image size

The size of each image can be set by using the `image:width` or `image:w` attributes in the image tag. For example, the 
following will cause the image to take up 50% of the terminal width:

```markdown
![image:width:50%](image.png)
```

The image will always be scaled to preserve its aspect ratio and it will not be allowed to overflow vertically nor 
horizontally.

## Protocol detection

By default the image protocol to be used will be automatically detected. In cases where this detection fails, you can 
set it manually via the `--image-protocol` parameter or by setting it in the [config 
file](../configuration/settings.md#preferred-image-protocol).
