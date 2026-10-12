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
    let
      lib = inputs.nixpkgs.lib;
      runners = {
        aarch64-darwin = "macos-15";
        aarch64-linux = "ubuntu-24.04-arm";
        x86_64-linux = "ubuntu-24.04";
      };
      # Discover derivations recursively so new outputs need no workflow edits.
      discover =
        path: value:
        if lib.isDerivation value then
          [
            {
              attr = lib.concatStringsSep "." path;
              system = builtins.elemAt path 1;
            }
          ]
        else if builtins.isAttrs value then
          lib.concatMap (name: discover (path ++ [ name ]) value.${name}) (builtins.attrNames value)
        else
          [ ];
      builds = lib.groupBy (entry: entry.system) (
        discover [ ] (lib.getAttrs [ "packages" "checks" "devShells" "formatter" ] self)
      );
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = builtins.attrNames runners;
      perSystem = import ./nix { inherit (inputs) crane; };
      flake.githubActions.matrix.include = lib.mapAttrsToList (system: entries: {
        inherit system;
        runner = runners.${system};
        attrs = map (entry: entry.attr) entries;
      }) builds;
    };
}
