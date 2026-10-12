{ pkgs, ... }:
let
  model = pkgs.callPackage ./model.nix { };
  runtime = pkgs.callPackage ./runtime.nix { };
  defingerprinter = pkgs.callPackage ./package.nix { inherit model runtime; };
  book = pkgs.callPackage ./book.nix { };
  rustdoc = pkgs.callPackage ./rustdoc.nix { };
  site = pkgs.callPackage ./site.nix { inherit book rustdoc; };
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
      book
      rustdoc
      site
      ;
    default = defingerprinter;
  };

  checks = {
    inherit book site;
    c-abi =
      pkgs.runCommand "defingerprinter-c-abi-check"
        {
          nativeBuildInputs = [ pkgs.stdenv.cc ];
        }
        ''
          $CC -std=c11 -Wall -Wextra -Werror ${../tests/c_abi.c} \
            -I${defingerprinter}/include -L${defingerprinter}/lib \
            -Wl,-rpath,${defingerprinter}/lib -ldefingerprinter -pthread -o smoke
          ./smoke
          touch "$out"
        '';
    cffi =
      pkgs.runCommand "defingerprinter-cffi-check"
        {
          nativeBuildInputs = [ sbcl ];
          DEFINGERPRINTER_LIBRARY = "${defingerprinter}/lib/libdefingerprinter${pkgs.stdenv.hostPlatform.extensions.sharedLibrary}";
        }
        ''
          export HOME="$TMPDIR/home" XDG_CACHE_HOME="$TMPDIR/cache"
          mkdir -p "$HOME" "$XDG_CACHE_HOME"
          sbcl --non-interactive --eval '(require :asdf)' \
            --eval '(asdf:load-asd #P"${defingerprinter}/share/common-lisp/source/defingerprinter/defingerprinter.asd")' \
            --eval '(asdf:test-system "defingerprinter")'
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
