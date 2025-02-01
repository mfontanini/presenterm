# PDF export

Presentations can be converted into PDF by using a [helper tool](https://github.com/mfontanini/presenterm-export). You 
can install it by running:

```bash
pip install presenterm-export
```

> [!tip]
> Make sure that `presenterm-export` works by running `presenterm-export --version` before attempting to generate a PDF 
> file. If you get errors related to _weasyprint_, follow their [installation instructions](https://doc.courtbouillon.org/weasyprint/stable/first_steps.html) to ensure you meet all of their 
> dependencies. This has otherwise caused issues in macOS.

The only external dependency you'll need is [tmux](https://github.com/tmux/tmux/). After you've installed both of these, 
simply run _presenterm_ with the `--export-pdf` parameter to generate the output PDF:

```bash
presenterm --export-pdf examples/demo.md
```

The output PDF will be placed in `examples/demo.pdf`. 

> [!note]
> If you're using a separate virtual env to install _presenterm-export_ just make sure you activate it before running 
> _presenterm_ with the `--export-pdf` parameter.

## Page sizes

The size of each page in the generated PDF will depend on the size of your terminal. Make sure to adjust accordingly 
before running the command above, and not to resize it while the generation is happening to avoid issues.

## Active tmux sessions bug

Because of a [bug in tmux <= 3.5a](https://github.com/tmux/tmux/issues/4268), exporting a PDF while having other tmux
sessions running and attached will cause the size of the output PDF to match the size of those other sessions rather 
than the size of the terminal you're running _presenterm_ in. The workaround is to only have one attached tmux session
and to run the PDF export from that session.

## How it works

The conversion into PDF format is pretty convoluted. If you'd like to learn more visit 
[presenterm-export](https://github.com/mfontanini/presenterm-export)'s repo.
