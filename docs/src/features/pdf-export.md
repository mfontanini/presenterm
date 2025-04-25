# Exporting presentations in PDF format

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

## PDF page size

By default, the size of each page in the generated PDF will depend on the size of your terminal. 

If you would like to instead configure the dimensions by hand, set the `export.dimensions` key in the configuration file 
as described in the [settings page](../configuration/settings.md#pdf-export-size).

## Pause behavior

See the [settings page](../configuration/settings.md#pause-behavior) to learn how to configure the behavior of pauses in 
generated PDFs.
