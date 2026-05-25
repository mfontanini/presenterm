# Configuration

_presenterm_ allows you to customize its behavior via a configuration file. This file is stored, along with all of your 
custom themes, in the following directories:

* `$XDG_CONFIG_HOME/presenterm/` if that environment variable is defined, otherwise:
* `~/.config/presenterm/` in Linux or macOS.
* `~/Library/Application Support/presenterm/` in macOS.
* `~/AppData/Roaming/presenterm/config/` in Windows.

The configuration file will be looked up automatically in the directories above under the name `config.yaml`. e.g. on 
Linux you should create it under `~/.config/presenterm/config.yaml`. You can also specify a custom path to this file 
when running _presenterm_ via the `--config-file` parameter or the `PRESENTERM_CONFIG_FILE` environment variable.

A [sample configuration file](https://github.com/mfontanini/presenterm/blob/master/config.sample.yaml) is provided in 
the repository that you can use as a base.

# Configuration schema

A JSON schema that defines the configuration file's schema is available to be used with YAML language servers such as
[yaml-language-server](https://github.com/redhat-developer/yaml-language-server).

Include the following line at the beginning of your configuration file to have your editor pull in autocompletion 
suggestions and docs automatically:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/mfontanini/presenterm/master/config-file-schema.json
```
