{
  mkShellNoCC,
  rustc,
  cargo,
  rustfmt,
  clippy,
}:
mkShellNoCC {
  packages = [
    rustc
    cargo

    # Tools
    rustfmt
    clippy
  ];

  strictDeps = true;
}
