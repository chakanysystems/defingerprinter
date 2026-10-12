{ lib, craneLib }:
let
  cargoFiles = lib.fileset.unions [
    ../Cargo.toml
    ../Cargo.lock
    ../src
    ../tests
    ../examples
    ../LICENSE
    ../ATTRIBUTION.md
    ../licenses
  ];
  src = lib.fileset.toSource {
    root = ../.;
    fileset = cargoFiles;
  };
  commonArgs = {
    inherit src;
    strictDeps = true;
    cargoExtraArgs = "--locked --offline";
    cargoVendorDir = craneLib.vendorCargoDeps { inherit src; };
  };
in
{
  inherit cargoFiles commonArgs;
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
}
