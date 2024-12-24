## PDF export

Presentations can be converted into PDF by using a [helper tool](https://github.com/mfontanini/presenterm-export). You 
can install it by running:

```shell
pip install presenterm-export
```

> **Note**: make sure that `presenterm-export` works by running `presenterm-export --version` before attempting to 
> generate a PDF file. If you get errors related to _weasyprint_, follow their [installation 
> instructions](https://doc.courtbouillon.org/weasyprint/stable/first_steps.html) to ensure you meet all of their 
> dependencies. This has otherwise caused issues in macOS.

The only external dependency you'll need is [tmux](https://github.com/tmux/tmux/). After you've installed both of these, 
simply run _presenterm_ with the `--export-pdf` parameter to generate the output PDF:

```shell
presenterm --export-pdf examples/demo.md
```

The output PDF will be placed in `examples/demo.pdf`. **The size of each page will depend on the size of your terminal**
so make sure to adjust accordingly before running the command above. 

> Note: if you're using a separate virtual env to install _presenterm-export_ just make sure you activate it before 
> running _presenterm_ with the `--export-pdf` parameter.

### Active tmux sessions bug

Because of a [bug in tmux <= 3.5a](https://github.com/tmux/tmux/issues/4268), exporting a PDF while having other tmux
sessions running and attached will cause the size of the output PDF to match the size of those other sessions rather 
than the size of the terminal you're running _presenterm_ in. The workaround is to only have one attached tmux session
and to run the PDF export from that session.

### How it works

The conversion into PDF format is pretty convoluted. If you'd like to learn more visit 
[presenterm-export](https://github.com/mfontanini/presenterm-export)'s repo.
