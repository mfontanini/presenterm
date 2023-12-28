# Installation

There's different ways to install _presenterm_.

## Pre-built binaries (recommended)

The recommended way to install _presenterm_ is to download the latest pre-built version for 
your system from the [releases](https://github.com/mfontanini/presenterm/releases) page.

## Install via cargo

Alternatively, download [rust](https://www.rust-lang.org/) and run:

```shell
cargo install presenterm
```

## Latest unreleased version

To run the latest unreleased version clone the repo, then run:

```shell
cargo build --release
```

The output binary will be in `./target/release/presenterm`.

## Nix

To install _presenterm_ using the Nix package manager run:

```shell
nix-env -iA nixos.presenterm    # for nixos
nix-env -iA nixpkgs.presenterm  # for non-nixos
```

Or, you can install it by adding the following to your configuration.nix if you are on NixOS

```nix
environment.systemPackages = [
  pkgs.presenterm
];
```

Alternatively if you're a Nix user using flakes you can run:

```shell
nix run nixpkgs#presenterm            # to run from nixpkgs
nix run github:mfontanini/presenterm  # to run from github repo
```

For more information see 
[nixpkgs](https://search.nixos.org/packages?channel=unstable&show=presenterm&from=0&size=50&sort=relevance&type=packages&query=presenterm)
