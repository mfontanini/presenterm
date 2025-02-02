# Installation

_presenterm_ works on Linux, macOS, and Windows and can be installed in different ways:

## Pre-built binaries (recommended)

The recommended way to install _presenterm_ is to download the latest pre-built version for 
your system from the [releases](https://github.com/mfontanini/presenterm/releases) page.

## Install via cargo

Alternatively, download [rust](https://www.rust-lang.org/) and run:

```bash
cargo install --locked presenterm
```

## Latest unreleased version

To install from the latest source code run:

```bash
cargo install --git https://github.com/mfontanini/presenterm
```

## macOS

Install the latest version in macOS via [brew](https://formulae.brew.sh/formula/presenterm) by running:

```bash
brew install presenterm
```

## Nix

To install _presenterm_ using the Nix package manager run:

```bash
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
[nixpkgs](https://search.nixos.org/packages?channel=unstable&show=presenterm&from=0&size=50&sort=relevance&type=packages&query=presenterm).

## Arch Linux

_presenterm_ is available in the [official repositories](https://archlinux.org/packages/extra/x86_64/presenterm/). You can use [pacman](https://wiki.archlinux.org/title/pacman) to install as follows:

```bash
pacman -S presenterm
```

#### Binary release

Alternatively, you can use any AUR helper to install the upstream binaries:

```bash
paru/yay -S presenterm-bin
```

#### Building from git

```bash
paru/yay -S presenterm-git
```

## Windows

Install the latest version in Scoop via [Scoop](https://scoop.sh/#/apps?q=presenterm&id=a462290f824b50f180afbaa6d8c7c1e6e0952e3a) by running:

```powershell
scoop install presenterm
```
