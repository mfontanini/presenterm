{
  description = "A terminal slideshow tool";

  inputs.nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz";

  outputs =
    { nixpkgs, self }:
    let
      inherit (nixpkgs.lib) genAttrs systems;
      forEachSystem = genAttrs systems.flakeExposed;
      pkgsForEach = nixpkgs.legacyPackages;
      rev = self.shortRev or self.dirtyShortRev or "unknown";
    in
    {
      packages = forEachSystem (system: {
        presenterm = pkgsForEach.${system}.callPackage ./package.nix { inherit rev; };
        default = self.packages.${system}.presenterm;
      });

      devShells = forEachSystem (system: {
        default = pkgsForEach.${system}.callPackage ./shell.nix { };
      });
    };
}
