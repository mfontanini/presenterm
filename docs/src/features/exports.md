# Exporting presentations

Presentations can be exported to PDF and HTML, to allow easily sharing the slide deck at the end of a presentation.

## PDF

Presentations can be converted into PDF by using [weasyprint](https://pypi.org/project/weasyprint/). Follow their 
[installation instructions](https://doc.courtbouillon.org/weasyprint/stable/first_steps.html) since it may require you 
to install extra dependencies for the tool to work.

> [!note]
> If you were using _presenterm-export_ before it was deprecated, that tool already required _weasyprint_ so it is 
> already installed in whatever virtual env you were using and there's nothing to be done.


After you've installed _weasyprint_, run _presenterm_ with the `--export-pdf` parameter to generate the output PDF:

```bash
presenterm --export-pdf examples/demo.md
```

The output PDF will be placed in `examples/demo.pdf`. Alternatively you can use the `--output` flag to specify where you 
want the output file to be written to.

> [!note]
> If you're using a separate virtual env to install _weasyprint_ just make sure you activate it before running 
> _presenterm_ with the `--export-pdf` parameter.

> [!note]
> If you have [uv](https://github.com/astral-sh/uv) installed you can simply run: 
> ```bash
> uv run --with weasyprint presenterm --export-pdf examples/demo.md
> ```

## HTML

Similarly, using the `--export-html` parameter allows generating a single self contained HTML file that contains all 
images and styles embedded in it. As opposed to PDF exports, this requires no extra dependencies:

```bash
presenterm --export-html examples/demo.md
```

The output file will be placed in `examples/demo.html` but this behavior can be configured via the `--output` flag just 
like for PDF exports.

## Serving over HTTP

Instead of writing the HTML to a file, _presenterm_ can serve it directly over HTTP so it can be viewed in a web browser,
for example to share it with your audience or to open it on another device. Use the `--serve` parameter:

```bash
presenterm --serve examples/demo.md
```

This renders the same single self contained HTML document used by `--export-html` — with all images embedded as inline 
base64 data URIs and all styles inline — and serves it. No extra dependencies are required and nothing is written to 
disk. The presentation is navigated in the browser with the arrow keys (or by swiping on touch devices).

By default the server binds to `127.0.0.1:8080`. Use `--serve-address` to bind to a different address and port, for 
example to make the presentation reachable from other machines on your network:

```bash
presenterm --serve --serve-address 0.0.0.0:3000 examples/demo.md
```

> [!note]
> The HTML is rendered once when the server starts, so restart _presenterm_ to pick up changes to the presentation.

# Configurable behavior

See the [settings page](../configuration/settings.md#presentation-exports) to see all the configurable behavior around 
presentation exports.

