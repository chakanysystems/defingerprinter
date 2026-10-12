{
  description = "Chakany Systems defingerprinter: offline Rampart C ABI redaction";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/DeterminateSystems/nixpkgs-weekly/0.1";
    crane.url = "github:ipetkov/crane";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];

      perSystem = import ./nix/per-system.nix { inherit (inputs) crane; };

      flake.githubActions.matrix = import ./nix/ci-matrix.nix {
        lib = inputs.nixpkgs.lib;
        outputs = self;
      };
    };
}
