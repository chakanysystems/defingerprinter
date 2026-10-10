# Reduced CPU runtime

Defingerprinter uses ONNX Runtime 1.24.2, reduced to the original pinned Rampart q4 graph. Keep graph optimization disabled: graph rewrites can introduce kernels outside the operator list.

## Nix build

```sh
nix build .#runtime
nix build .#default
nix flake check
```

`nix/runtime.nix` verifies the official source archive and immutable dependency archives. Nix fetches dependencies separately; CMake dependency downloads are disabled with `FETCHCONTENT_FULLY_DISCONNECTED`. The original ONNX loader, exceptions and contrib `com.microsoft::MatMulNBits` are retained. The configuration uses MinSizeRel, LTO, reduced operator registration, disabled ML/training/RTTI, and CPU execution only. It supports ONNX Runtime C API 22, as requested by the Rust binding.

C++ module dependency scanning is disabled because no modules are used and `clang-scan-deps` bypasses Nix compiler-wrapper include flags. Darwin minimum-version hook is 13.3, but the pinned SDK/toolchain selects macOS 14.0 during configuration. Treat the Nix output as requiring macOS 14; the older manual artifact below used a separate macOS 13 target.

Nix model fetches are fixed-output downloads pinned to revision `b1993e4e68b082835b80ffc65acc03325ea2e501`. All four files also have compile-time SHA256 checks in Rust. Runtime initialization is lazy, with one CPU thread, disabled spinning, graph optimization and telemetry. Model and runtime paths are compiled into the Nix library.

C ABI and CFFI flake checks load the actual model. Network isolation during Nix builds depends on the host's sandbox configuration; validate runtime inference separately under a network-denying sandbox when certifying deployment.

On 2026-10-10, the verified Darwin arm64 Nix runtime occupied 9,811,888 bytes (9.36 MiB). The complete Defingerprinter runtime closure, including model and libiconv, was 73.6 MiB. Compiler source-path remapping and removal of an unused protobuf CMake export reduced the closure from 150.5 MiB; dependency source archives are explicitly forbidden as runtime references. C ABI and CFFI inference passed with networking denied.

## Optional manual macOS arm64 bundle

Install Cargo, Python 3, CMake, Xcode command-line tools, and `hf` on the build machine. Obtain the four pinned model files:

```sh
hf download nationaldesignstudio/rampart \
  --revision b1993e4e68b082835b80ffc65acc03325ea2e501 \
  --include config.json tokenizer.json tokenizer_config.json onnx/model_q4.onnx \
  --local-dir models/rampart
JOBS=12 sh scripts/build-runtime.sh
sh scripts/package.sh
```

To reuse another local model directory, set `DEFINGERPRINTER_MODEL_DIR` to its absolute path. Downloads and compilation logs go under ignored `target/native-runtime/`. The build script creates a Python virtual environment, checks source/model hashes, regenerates the operator list using upstream tooling, and compares it with `nix/rampart-operators.config`. Dependency archive hashes and Python package versions are pinned by the script.

The package script builds the executable and C shared library, copies headers, CFFI sources, model assets and dependency notices, verifies actual inference, and emits `target/defingerprinter-darwin-arm64.tar.gz` with `SHA256SUMS`. The CLI resolves portable assets relative to its executable; C and CFFI callers must supply absolute artifact paths for this manual bundle. End users need no compiler, Node or Python. Ad-hoc signatures are development signatures; apply distribution signing and notarization before shipping.

## Historical manual artifact

The pre-extraction macOS arm64 runtime built on 2026-10-10 occupied 6,360,128 bytes (6.07 MiB), versus 44,589,928 bytes (42.52 MiB) for the comparison Node runtime. The original q4 model is 14,693,379 bytes (14.01 MiB). These are manual-build measurements, not Nix closure sizes.

Manual runtime SHA256:

```text
6723be987f6ef389f18841da98212ebf7f1faafab016f438662e0410ec8ec03a
```

C API 22 inference passed at sequence lengths 4, 16, 128 and 512 with finite logits and shape `[1, sequence_length, 35]`. Full tokenizer, NER-window and structured-redaction parity were checked against the pinned JavaScript reference during extraction. Compiler/SDK changes affect binary fingerprints; retain each build's provenance and test the final artifact.

## References

- [Custom builds](https://onnxruntime.ai/docs/build/custom.html)
- [Reduced operator configuration](https://onnxruntime.ai/docs/reference/operators/reduced-operator-config-file.html)
- [Pinned runtime source](https://github.com/microsoft/onnxruntime/tree/v1.24.2)
