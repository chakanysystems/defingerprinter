{
  runCommand,
  book,
  rustdoc,
}:
let
  projectBook = book.override { siteUrl = "/defingerprinter/"; };
in
runCommand "defingerprinter-site" { } ''
  mkdir -p "$out/api"
  cp -R ${projectBook}/. "$out/"
  cp -R ${rustdoc}/. "$out/api/"
  cp ${../docs/api-index.html} "$out/api/index.html"
  touch "$out/.nojekyll"
  test -f "$out/index.html"
  test -f "$out/api/defingerprinter/index.html"
''
