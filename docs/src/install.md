# Installing _presenterm_

_presenterm_ works on Linux, macOS, and Windows and can be installed in different ways:

#### Binary

The recommended way to install _presenterm_ is to download the latest pre-built version for your system from the 
[releases page](https://github.com/mfontanini/presenterm/releases).

#### cargo-binstall

If you're a [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) user:

```bash
cargo binstall presenterm
```

#### From source

Alternatively, build from source by downloading [rust](https://www.rust-lang.org/) and running:

```bash
cargo install --locked presenterm
```

## Latest unreleased version

The latest unreleased version can be installed either in binary form or by building it from source.

#### Binary

The nightly pre-build binary can be downloaded from 
[github](https://github.com/mfontanini/presenterm/releases/tag/nightly). Keep in mind this is built once a day at 
midnight UTC so if you need code that has been recently merged you may have to wait a few hours.

#### From source

```bash
cargo install --locked --git https://github.com/mfontanini/presenterm
```

# Community maintained packages

The community maintains packages for various operating systems and linux distributions and can be installed in the 
following ways:

## macOS

Install the latest version in macOS via [brew](https://formulae.brew.sh/formula/presenterm) by running:

```bash
brew install presenterm
```

The latest unreleased version can be built via brew by running:

```bash
brew install --head presenterm
```

## Nix

To install _presenterm_ using the Nix package manager run:

```bash
nix-env -iA nixos.presenterm    # for nixos
nix-env -iA nixpkgs.presenterm  # for non-nixos
```

#### NixOS

Add the following to your `configuration.nix` if you are on NixOS

```nix
environment.systemPackages = [
  pkgs.presenterm
];
```

#### Flakes

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

#### Binary

Alternatively, you can use any AUR helper to install the upstream binaries:

```bash
paru/yay -S presenterm-bin
```

#### From source

```bash
paru/yay -S presenterm-git
```

## Windows

#### Scoop

Install the [latest version](https://scoop.sh/#/apps?q=presenterm&id=a462289f824b50f180afbaa6d8c7c1e6e0952e3a) via scoop 
by running:

```powershell
scoop install main/presenterm
```

#### Winget

Alternatively, you can install via [WinGet](https://github.com/microsoft/winget-cli) by running:

```powershell
winget install --id=mfontanini.presenterm  -e
```
