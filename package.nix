{
  lib,
  rustPlatform,
  rev ? "dirty",
}:
let
  cargoToml = lib.importTOML ./Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "presenterm";
  version = "${cargoToml.package.version}-${rev}";

  src =
    let
      fs = lib.fileset;
    in
    fs.toSource {
      root = ./.;
      fileset = fs.unions [
        ./build.rs
        ./Cargo.toml
        ./Cargo.lock
        ./src
        ./themes
        ./bat
        ./executors.yaml
      ];
    };

  cargoLock.lockFile = ./Cargo.lock;
}
