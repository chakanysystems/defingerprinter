{ lib, stdenv, rustPlatform, jq, model, runtime }:
rustPlatform.buildRustPackage {
  pname = "defingerprinter";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml ../Cargo.lock ../src ../tests ../include
      ../LICENSE ../ATTRIBUTION.md ../licenses
      ../defingerprinter.asd ../lisp
      ../scripts/collect-rust-licenses.sh
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;
  nativeBuildInputs = [ jq ];
  # Compile-time trusted defaults, not process-environment overrides.
  DEFINGERPRINTER_MODEL_DIR = "${model}";
  DEFINGERPRINTER_RUNTIME = "${runtime}/lib/libonnxruntime${stdenv.hostPlatform.extensions.sharedLibrary}";
  doCheck = true;
  postCheck = ''sh tests/notices.sh'';
  postInstall = ''
    mkdir -p "$out/include" "$out/share/licenses/defingerprinter"
    cp -R include/. "$out/include/"
    cp ATTRIBUTION.md "$out/share/licenses/defingerprinter/"
    cp LICENSE "$out/share/licenses/defingerprinter/"
    cp -R licenses/. "$out/share/licenses/defingerprinter/"
    sh scripts/collect-rust-licenses.sh "$out/share/licenses/defingerprinter"
    bindingDir="$out/share/common-lisp/source/defingerprinter"
    mkdir -p "$bindingDir/tests"
    cp defingerprinter.asd "$bindingDir/"
    cp LICENSE "$bindingDir/"
    cp -R lisp "$bindingDir/"
    cp tests/cffi.lisp "$bindingDir/tests/"
    test -f "$out/lib/libdefingerprinter${stdenv.hostPlatform.extensions.sharedLibrary}"
  '' + lib.optionalString stdenv.hostPlatform.isDarwin ''
    install_name_tool -id "$out/lib/libdefingerprinter.dylib" "$out/lib/libdefingerprinter.dylib"
  '';
  passthru = { inherit model runtime; };
  meta = {
    description = "Offline Rampart redaction library with a consent-gated C ABI";
    mainProgram = "defingerprinter";
    license = [ lib.licenses.mit lib.licenses.cc-by-40 ];
    platforms = [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ];
  };
}
