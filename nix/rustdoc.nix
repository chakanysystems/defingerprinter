{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "defingerprinter-rustdoc";
  version = "0.2.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../LICENSE
      ../ATTRIBUTION.md
      ../licenses
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;
  RUSTDOCFLAGS = "-D warnings";
  buildPhase = ''
    runHook preBuild
    cargo doc --offline --locked --no-deps --lib
    runHook postBuild
  '';
  doCheck = true;
  checkPhase = ''
    runHook preCheck
    cargo test --offline --locked --doc
    runHook postCheck
  '';
  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -R target/doc/. "$out/"
    cp LICENSE ATTRIBUTION.md "$out/"
    cp -R licenses "$out/"
    runHook postInstall
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
