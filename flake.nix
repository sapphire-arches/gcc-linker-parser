{
  inputs = {
    fenix = {
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
      fenix,
      nixpkgs,
    }:
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          name = "linker-parser-dev";

          buildInputs = with pkgs; [
            (fenix.complete.withComponents [
            "cargo" "rust-src" "rustc" "rustfmt"
            ])
            rust-analyzer
          ];
        };
      }
    ));
}
