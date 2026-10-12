{
  lib,
  craneLib,
  cargo,
}:
craneLib.cargoDoc (
  cargo.commonArgs
  // {
    inherit (cargo) cargoArtifacts;
    cargoDocExtraArgs = "--no-deps --lib";
    RUSTDOCFLAGS = "-D warnings";
    doCheck = true;
    checkPhaseCargoCommand = "cargo test --offline --locked --profile release --doc";
    postInstall = ''
      cp -R "$out/share/doc/." "$out/"
      rm -r "$out/share"
      cp LICENSE ATTRIBUTION.md "$out/"
      cp -R licenses "$out/"
    '';
    meta = {
      description = "Rust API documentation and checked doctests for Defingerprinter";
      license = [
        lib.licenses.mit
        lib.licenses.cc-by-40
      ];
      platforms = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
    };
  }
)
