{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    fenix.url = "github:nix-community/fenix";
  };

  outputs = { self, nixpkgs, utils, naersk, fenix }: utils.lib.eachDefaultSystem
    (system:
      let
        name = "masklint";
        version = "latest";
        # https://discourse.nixos.org/t/using-nixpkgs-legacypackages-system-vs-import/17462/7
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-eMJethw5ZLrJHmoN2/l0bIyQjoTX1NsvalWSscTixpI=";
        };
        naersk' = naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        };
      in
      with pkgs;
      rec {
        packages = {
          default = packages.${name};
          "${name}" = naersk'.buildPackage {
            inherit name version;
            src = ./.;
          };
        };

        apps = {
          default = apps.${name};
          "${name}" = utils.lib.mkApp {
            drv = packages.default;
            exePath = "/bin/${name}";
          };
        };

        devShell = mkShellNoCC {
          packages = [
            # rust
            rustup
            cargo-audit
            cargo-outdated
            cargo-cross
            cargo-edit

            mask
            yq-go
            ripgrep
            fd
            goreleaser
            svu
            commitlint
            syft
            cosign

            # shells
            shellcheck

            # python
            python311
            python311Packages.pylint

            # ruby
            ruby_3_2
            rubyPackages_3_2.rubocop
          ];


          # https://github.com/openebs/mayastor-control-plane/blob/develop/shell.nix
          NODE_PATH = "${nodePackages."@commitlint/config-conventional"}/lib/node_modules";
          # see https://github.com/cross-rs/cross/issues/1241
          CROSS_CONTAINER_OPTS = "--platform linux/amd64";
        };
      }
    );
}
