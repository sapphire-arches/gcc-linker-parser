{
  inputs = {
    fenix-module = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs";
  };

  outputs =
    {
      self,
      flake-utils,
      fenix-module,
      nixpkgs,
    }:
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix-module.overlays.default ];
        };
      in
      {
        inherit pkgs;
        devShells.default = pkgs.mkShell {
          name = "linker-parser-dev";

          buildInputs = with pkgs; [
            (fenix.stable.withComponents [
              "cargo"
              "rust-src"
              "rustc"
              "rustfmt"
            ])
            rust-analyzer-nightly
          ];
        };
      }
    ));
}
