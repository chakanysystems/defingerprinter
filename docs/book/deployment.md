# Deployment

## Nix package layout

```text
bin/defingerprinter
lib/libdefingerprinter.dylib           # .so on Linux
include/defingerprinter.h
share/common-lisp/source/defingerprinter/
share/licenses/defingerprinter/
```

Model and runtime are separate store outputs referenced by immutable compile-time paths. Copy or deploy the **complete Nix closure**, rather than moving just the executable or shared library. Obtain the closure's references with your usual Nix deployment tooling.

```sh
nix build .#default
nix flake check
```

C ABI and CFFI flake checks perform real inference. Run representative synthetic integration checks in the final deployment environment too. Build-time sandbox isolation and runtime network-denial testing are separate concerns.

## Import from another flake

```nix
{
  inputs.defingerprinter.url = "git+file:///absolute/path/to/defingerprinter?ref=trunk";
  outputs = { self, defingerprinter }: {
    packages.aarch64-darwin.default =
      defingerprinter.packages.aarch64-darwin.default;
  };
}
```

Pin the actual revision in your consumer lockfile. `trunk` is this repository's main bookmark. No published remote is assumed.

## Compatibility

Library release 0.1.0 uses ABI 2. Version 2 permits compatible shared-library replacements on the same platform and architecture. Check `defingerprinter_abi_version()` before creating contexts. Incompatible future ABIs require a new version. Model and redaction-policy behavior are separately pinned.

The flake exposes `default`, `defingerprinter`, `model`, `runtime`, and `book` packages for `aarch64-darwin`, `aarch64-linux`, and `x86_64-linux`. Darwin arm64 is locally tested; validate Linux targets natively. The pinned Nix Darwin runtime requires macOS 14 according to the selected SDK/toolchain.

## Manual bundles

The optional packaging scripts prepare a macOS arm64 archive with native artifacts and notices. The CLI can resolve portable defaults relative to its executable. C and CFFI callers must pass absolute artifact paths for this bundle. See [the reduced runtime build](runtime.md) for the exact build procedure and provenance.

Apply distribution signing and notarization as appropriate. Validate the final signed package, matching architecture, loader paths, actual inference, and process cleanup.

## Licenses and notices

The original library code is MIT-licensed. Incorporated upstream code and model artifacts have their own licenses and attribution requirements. Review `LICENSE`, `ATTRIBUTION.md`, and `licenses/` in the source checkout, and preserve the installed notices when distributing a package or closure.
