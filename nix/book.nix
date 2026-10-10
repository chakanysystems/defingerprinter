{
  lib,
  runCommand,
  mdbook,
  siteUrl ? null,
}:
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
  ''
