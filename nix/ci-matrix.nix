{ lib, outputs }:
let
  runners = {
    aarch64-darwin = "macos-15";
    aarch64-linux = "ubuntu-24.04-arm";
    x86_64-linux = "ubuntu-24.04";
  };
  # Find derivations recursively so new output names need no workflow edits.
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
    discover [ ] (lib.getAttrs [ "packages" "checks" "devShells" "formatter" ] outputs)
  );
in
{
  include = lib.mapAttrsToList (system: entries: {
    inherit system;
    runner = runners.${system} or (throw "No GitHub Actions runner configured for ${system}");
    attrs = map (entry: entry.attr) entries;
  }) builds;
}
