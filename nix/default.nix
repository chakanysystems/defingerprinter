{ crane }:
{ pkgs, ... }:
let
  inherit (pkgs)
    lib
    stdenv
    fetchurl
    runCommand
    mdbook
    jq
    ;
  craneLib = crane.mkLib pkgs;
  runtime = pkgs.callPackage ./runtime.nix { };

  model =
    let
      revision = "b1993e4e68b082835b80ffc65acc03325ea2e501";
      hashes = {
        "config.json" = "003b84bbcd489f5e782fe5cad8f3249c3653ec880089abb1ccc398a0d895e3e6";
        "tokenizer.json" = "98ade711428b42a1b5343c403a73344535e92de8e19359cdb567ef34da210259";
        "tokenizer_config.json" = "0088a6f8bcdd4014184fb068b83ebb12896a9db2bb269a71f73de83fef24bceb";
        "onnx/model_q4.onnx" = "9f27d24949b0581701071ea5ef522d77ccd3f50c525cc91eac4d265b0fc2afe5";
      };
      files = lib.mapAttrs (
        name: sha256:
        fetchurl {
          url = "https://huggingface.co/NationalDesignStudio/rampart/resolve/${revision}/${name}";
          inherit sha256;
          name = builtins.baseNameOf name;
        }
      ) hashes;
    in
    runCommand "defingerprinter-rampart-model-${builtins.substring 0 12 revision}"
      {
        meta = {
          description = "Original pinned Rampart quantized ONNX model and tokenizer";
          license = lib.licenses.cc-by-40;
        };
        passthru = { inherit revision hashes; };
      }
      (
        ''
          mkdir -p "$out/onnx" "$out/licenses"
          cp ${../ATTRIBUTION.md} "$out/ATTRIBUTION.md"
          cp ${../licenses/CC-BY-4.0.txt} "$out/licenses/CC-BY-4.0.txt"
        ''
        + lib.concatStringsSep "\n" (
          lib.mapAttrsToList (name: file: ''
            cp ${file} "$out/${name}"
          '') files
        )
      );

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
  # Default features build the Tract backend and need no native runtime.
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
  ortArgs = commonArgs // {
    cargoExtraArgs = "--locked --offline --no-default-features --features ort";
  };
  ortArtifacts = craneLib.buildDepsOnly ortArgs;
  bothArgs = commonArgs // {
    cargoExtraArgs = "--locked --offline --features ort";
  };
  bothArtifacts = craneLib.buildDepsOnly bothArgs;

  defingerprinterArgs =
    commonArgs
    // {
      inherit cargoArtifacts;
      src = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          cargoFiles
          ../include
          ../defingerprinter.asd
          ../lisp
          ../scripts/collect-rust-licenses.sh
        ];
      };
      nativeBuildInputs = [ jq ];
      # Compile-time trusted defaults, not process-environment overrides.
      DEFINGERPRINTER_MODEL_DIR = "${model}";
      outputs = [
        "out"
        "bin"
        "lib"
        "dev"
      ];
      doCheck = true;
      postCheck = "sh tests/notices.sh";
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
      ''
      + lib.optionalString stdenv.hostPlatform.isDarwin ''
        install_name_tool -id "$lib/lib/libdefingerprinter.dylib" "$out/lib/libdefingerprinter.dylib"
      ''
      + ''
        moveToOutput "bin" "$bin"
        moveToOutput "lib" "$lib"
        moveToOutput "include" "$dev"
        moveToOutput "share/common-lisp" "$dev"
      '';
      passthru = { inherit model runtime; };
      meta = {
        description = "Offline Rampart redaction library with a versioned C ABI";
        mainProgram = "defingerprinter";
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
    };

  defingerprinter = craneLib.buildPackage defingerprinterArgs;

  ort = craneLib.buildPackage (
    lib.recursiveUpdate defingerprinterArgs {
      cargoArtifacts = ortArtifacts;
      cargoExtraArgs = "--locked --offline --no-default-features --features ort";
      DEFINGERPRINTER_RUNTIME = "${runtime}/lib/libonnxruntime${stdenv.hostPlatform.extensions.sharedLibrary}";
    }
  );

  rustdoc = craneLib.cargoDoc (
    commonArgs
    // {
      inherit cargoArtifacts;
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
  );

  mkBook =
    siteUrl:
    let
      source = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          ../book.toml
          ../docs/book
          ../docs/runtime-build.md
          ../examples
        ];
      };
    in
    runCommand "defingerprinter-handbook"
      (
        {
          nativeBuildInputs = [ mdbook ];
          src = source;
        }
        // lib.optionalAttrs (siteUrl != null) {
          MDBOOK_OUTPUT__HTML__SITE_URL = siteUrl;
        }
      )
      ''
        cp -R "$src" source
        chmod -R u+w source
        cd source
        mdbook build --dest-dir "$out"
      '';

  book = mkBook null;
  projectBook = mkBook "/defingerprinter/";

  site = runCommand "defingerprinter-site" { } ''
    mkdir -p "$out/api"
    cp -R ${projectBook}/. "$out/"
    # Replace the handbook's API navigation page with generated rustdoc.
    chmod -R u+w "$out/api"
    cp -R ${rustdoc}/. "$out/api/"
    cp ${../docs/api-index.html} "$out/api/index.html"
    touch "$out/.nojekyll"
    test -f "$out/index.html"
    test -f "$out/api/defingerprinter/index.html"
  '';

  sbcl = pkgs.sbcl.withPackages (p: [
    p.cffi
    p.babel
    p.bordeaux-threads
  ]);
in
{
    packages = {
      inherit
        model
        runtime
        defingerprinter
        ort
        book
        rustdoc
        site
        ;
      default = defingerprinter;
    };

  checks = {
    inherit book site;
    clippy = craneLib.cargoClippy (
      bothArgs
      // {
        cargoArtifacts = bothArtifacts;
        cargoClippyExtraArgs = "--all-targets -- -D warnings";
      }
    );
    clippy-ort = craneLib.cargoClippy (
      ortArgs
      // {
        cargoArtifacts = ortArtifacts;
        cargoClippyExtraArgs = "--all-targets -- -D warnings";
      }
    );
    rustfmt = craneLib.cargoFmt { inherit src; };
    tract-parity = craneLib.mkCargoDerivation (
      bothArgs
      // {
        cargoArtifacts = bothArtifacts;
        pnameSuffix = "-tract-parity";
        buildPhaseCargoCommand = "cargo build --release --locked --offline --features ort --example tract_compare";
        doCheck = true;
        doInstallCargoArtifacts = false;
        RAMPART_MODEL_DIR = "${model}";
        checkPhaseCargoCommand = ''
          cargo test --release --locked --offline --features ort
          cargo test --release --locked --offline --features ort --test artifacts -- --ignored
            cargo test --release --locked --offline --features ort --test tract_upgrade -- --ignored
          target/release/examples/tract_compare ${model} ${runtime}/lib/libonnxruntime${stdenv.hostPlatform.extensions.sharedLibrary} | tee "$TMPDIR/tract-comparison.log"
          target/release/examples/tract_compare ${model}
        '';
        installPhaseCommand = ''
          mkdir -p "$out"
          cp "$TMPDIR/tract-comparison.log" "$out/results.txt"
        '';
      }
    );
    c-abi =
      pkgs.runCommand "defingerprinter-c-abi-check"
        {
          nativeBuildInputs = [ pkgs.stdenv.cc ];
        }
        ''
          $CC -std=c11 -Wall -Wextra -Werror ${../tests/c_abi.c} \
            -I${defingerprinter.dev}/include -L${defingerprinter.lib}/lib \
            -Wl,-rpath,${defingerprinter.lib}/lib -ldefingerprinter -pthread -o smoke
          ./smoke
          touch "$out"
        '';
    cffi =
      pkgs.runCommand "defingerprinter-cffi-check"
        {
          nativeBuildInputs = [ sbcl ];
      DEFINGERPRINTER_LIBRARY = "${defingerprinter.lib}/lib/libdefingerprinter${pkgs.stdenv.hostPlatform.extensions.sharedLibrary}";
        }
        ''
            export HOME="$TMPDIR/home" XDG_CACHE_HOME="$TMPDIR/cache"
            # nixpkgs lisp packages ship fasls built with an older SBCL.
            export ASDF_OUTPUT_TRANSLATIONS="/:$TMPDIR/cache/asdf"
            mkdir -p "$HOME" "$XDG_CACHE_HOME" "$TMPDIR/cache/asdf"
            sbcl --non-interactive --eval '(require :asdf)' \
              --eval '(asdf:load-asd #P"${defingerprinter.dev}/share/common-lisp/source/defingerprinter/defingerprinter.asd")' \
              --eval '(asdf:test-system "defingerprinter")' --quit
          touch "$out"
        '';
  };

  devShells = {
    default = pkgs.mkShell {
      packages = [
        pkgs.cargo
        pkgs.rustc
        pkgs.rustfmt
        pkgs.clippy
        pkgs.mdbook
      ];
      DEFINGERPRINTER_MODEL_DIR = "${model}";
      DEFINGERPRINTER_RUNTIME = "${runtime}/lib/libonnxruntime${pkgs.stdenv.hostPlatform.extensions.sharedLibrary}";
    };
    docs = pkgs.mkShell {
      packages = [
        pkgs.cargo
        pkgs.rustc
        pkgs.mdbook
        pkgs.stdenv.cc
      ];
    };
  };

  formatter = pkgs.nixfmt;
}
