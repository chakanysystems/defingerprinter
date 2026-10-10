{
  description = "Chakany Systems defingerprinter: offline Rampart C ABI redaction";
  inputs.nixpkgs.url = "https://flakehub.com/f/DeterminateSystems/nixpkgs-weekly/0.1";
  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ];
      eachSystem = nixpkgs.lib.genAttrs systems;
      forSystem = system:
        let
          pkgs = import nixpkgs { inherit system; };
          model = pkgs.callPackage ./nix/model.nix { };
          runtime = pkgs.callPackage ./nix/runtime.nix { };
          defingerprinter = pkgs.callPackage ./nix/package.nix { inherit model runtime; };
          book = pkgs.callPackage ./nix/book.nix { };
          sbcl = pkgs.sbcl.withPackages (p: [ p.cffi p.babel p.bordeaux-threads ]);
        in { inherit pkgs model runtime defingerprinter book sbcl; };
    in {
      packages = eachSystem (system:
        let p = forSystem system;
        in { inherit (p) model runtime defingerprinter book; default = p.defingerprinter; });
      checks = eachSystem (system:
        let p = forSystem system;
        in {
          book = p.book;
          c-abi = p.pkgs.runCommand "defingerprinter-c-abi-check" {
            nativeBuildInputs = [ p.pkgs.stdenv.cc ];
          } ''
            $CC -std=c11 -Wall -Wextra -Werror ${./tests/c_abi.c} \
              -I${p.defingerprinter}/include -L${p.defingerprinter}/lib \
              -Wl,-rpath,${p.defingerprinter}/lib -ldefingerprinter -pthread -o smoke
            ./smoke
            touch "$out"
          '';
          cffi = p.pkgs.runCommand "defingerprinter-cffi-check" {
            nativeBuildInputs = [ p.sbcl ];
            DEFINGERPRINTER_LIBRARY = "${p.defingerprinter}/lib/libdefingerprinter${p.pkgs.stdenv.hostPlatform.extensions.sharedLibrary}";
          } ''
            export HOME="$TMPDIR/home" XDG_CACHE_HOME="$TMPDIR/cache"
            mkdir -p "$HOME" "$XDG_CACHE_HOME"
            sbcl --non-interactive --eval '(require :asdf)' \
              --eval '(asdf:load-asd #P"${p.defingerprinter}/share/common-lisp/source/defingerprinter/defingerprinter.asd")' \
              --eval '(asdf:test-system "defingerprinter")'
            touch "$out"
          '';
        });
      devShells = eachSystem (system:
        let p = forSystem system;
        in {
          default = p.pkgs.mkShell {
            packages = [ p.pkgs.cargo p.pkgs.rustc p.pkgs.rustfmt p.pkgs.clippy p.pkgs.mdbook ];
            DEFINGERPRINTER_MODEL_DIR = "${p.model}";
            DEFINGERPRINTER_RUNTIME = "${p.runtime}/lib/libonnxruntime${p.pkgs.stdenv.hostPlatform.extensions.sharedLibrary}";
          };
          docs = p.pkgs.mkShell {
            packages = [ p.pkgs.cargo p.pkgs.rustc p.pkgs.mdbook p.pkgs.stdenv.cc ];
          };
        });
      formatter = eachSystem (system: (forSystem system).pkgs.nixfmt);
    };
}
