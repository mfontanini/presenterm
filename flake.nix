{
  description = "A terminal slideshow tool";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    flakebox = {
      url = "github:rustshop/flakebox?rev=41e88a8c6910829ec598ee356325e515de043541";
    };
  };

  outputs = { self, flake-utils, flakebox }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        projectName = "presenterm";

        flakeboxLib = flakebox.lib.${system} {
          config = {
            github.ci.buildOutputs = [ ".#ci.${projectName}" ];
          };
        };

        buildPaths = [
          "build.rs"
          "Cargo.toml"
          "Cargo.lock"
          ".cargo"
          "src"
          "themes"
          "bat"
          "executors"
        ];

        buildSrc = flakeboxLib.filterSubPaths {
          root = builtins.path {
            name = projectName;
            path = ./.;
          };
          paths = buildPaths;
        };

        multiBuild =
          (flakeboxLib.craneMultiBuild { }) (craneLib':
            let
              craneLib = (craneLib'.overrideArgs {
                pname = projectName;
                src = buildSrc;
                nativeBuildInputs = [ ];
              });
            in
            {
              ${projectName} = craneLib.buildPackage { };
            });
      in
      {
        packages.default = multiBuild.${projectName};

        legacyPackages = multiBuild;

        devShells = flakeboxLib.mkShells { };
      }
    );
}
