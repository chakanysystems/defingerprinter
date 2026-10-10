#!/bin/sh
# Build-machine only. End users receive the dylib, licenses, and native sidecar.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK="$ROOT/target/native-runtime"
VERSION=1.24.2
SOURCE_SHA=e4f44579f0bdd06d1cc7abdb2a6740dc06ac10699d502c0f8d9576203550bd4d
MODEL_SHA=9f27d24949b0581701071ea5ef522d77ccd3f50c525cc91eac4d265b0fc2afe5
SOURCE="$WORK/onnxruntime-$VERSION"
ARCHIVE="$WORK/onnxruntime-$VERSION.tar.gz"
MODEL="${DEFINGERPRINTER_MODEL_DIR:-$ROOT/models/rampart}/onnx/model_q4.onnx"
PACKAGE="$WORK/package"
PYTHON=${PYTHON:-python3}
JOBS=${JOBS:-12}

if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
  echo 'This recipe targets macOS arm64. Port and verify before packaging other platforms.' >&2
  exit 1
fi
for command in "$PYTHON" cmake clang curl tar shasum strip install_name_tool codesign; do
  command -v "$command" >/dev/null || { echo "Missing build tool: $command" >&2; exit 1; }
done
check_sha() {
  actual=$(shasum -a 256 "$1" | cut -d ' ' -f 1)
  [ "$actual" = "$2" ] || { echo "SHA256 mismatch: $1" >&2; exit 1; }
}
mkdir -p "$WORK"
check_sha "$MODEL" "$MODEL_SHA"
if [ ! -f "$ARCHIVE" ]; then
  curl --fail --location --retry 3 \
    "https://codeload.github.com/microsoft/onnxruntime/tar.gz/refs/tags/v$VERSION" \
    -o "$ARCHIVE.partial"
  check_sha "$ARCHIVE.partial" "$SOURCE_SHA"
  mv "$ARCHIVE.partial" "$ARCHIVE"
fi
check_sha "$ARCHIVE" "$SOURCE_SHA"
if [ ! -d "$SOURCE" ]; then
  tar -xzf "$ARCHIVE" -C "$WORK"
fi
if [ ! -x "$WORK/venv/bin/python" ]; then
  "$PYTHON" -m venv "$WORK/venv"
fi
PY="$WORK/venv/bin/python"
# Exact direct and transitive dependencies for the official ONNX/ORT build tools.
"$PY" -m pip install --disable-pip-version-check \
  onnx==1.20.1 flatbuffers==25.12.19 packaging==26.0 numpy==2.4.2 \
  ml_dtypes==0.6.0 protobuf==7.36.2 typing_extensions==4.16.0 \
  >"$WORK/pip.log" 2>&1
"$PY" "$SOURCE/tools/python/create_reduced_build_config.py" \
  "$MODEL" "$WORK/generated-operators.config"
grep -v '^#' "$WORK/generated-operators.config" >"$WORK/generated-operators.txt"
grep -v '^#' "$ROOT/nix/rampart-operators.config" >"$WORK/pinned-operators.txt"
cmp "$WORK/generated-operators.txt" "$WORK/pinned-operators.txt"

# No submodule commands: upstream CMake downloads immutable archives from its
# pinned deps.txt and checks their hashes. Avoid accidental Homebrew/Nix deps.
# Keep contrib ops (MatMulNBits), exceptions, and the ONNX loader. CPU only.
if ! "$PY" "$SOURCE/tools/ci_build/build.py" \
  --build_dir "$WORK/build" --config MinSizeRel --update --build \
  --parallel "$JOBS" --skip_tests --skip_submodule_sync --build_shared_lib \
  --disable_ml_ops --disable_rtti --enable_lto \
  --include_ops_by_config "$ROOT/nix/rampart-operators.config" \
  --cmake_generator 'Unix Makefiles' \
  --cmake_extra_defines FETCHCONTENT_TRY_FIND_PACKAGE_MODE=NEVER \
    onnxruntime_ENABLE_TRAINING_OPS=OFF onnxruntime_ENABLE_TRAINING=OFF \
    onnxruntime_USE_COREML=OFF onnxruntime_BUILD_UNIT_TESTS=OFF \
    CMAKE_OSX_ARCHITECTURES=arm64 CMAKE_OSX_DEPLOYMENT_TARGET=13.0 \
  >"$WORK/build.log" 2>&1; then
  tail -40 "$WORK/build.log" >&2
  exit 1
fi
mkdir -p "$PACKAGE/licenses"
cp "$WORK/build/MinSizeRel/libonnxruntime.$VERSION.dylib" "$PACKAGE/libonnxruntime.dylib"
strip -x "$PACKAGE/libonnxruntime.dylib"
install_name_tool -id '@rpath/libonnxruntime.dylib' "$PACKAGE/libonnxruntime.dylib"
codesign --force --sign - "$PACKAGE/libonnxruntime.dylib"
cp "$SOURCE/LICENSE" "$PACKAGE/licenses/onnxruntime-LICENSE"
cp "$SOURCE/ThirdPartyNotices.txt" "$PACKAGE/licenses/onnxruntime-ThirdPartyNotices.txt"
cp "$ROOT/nix/rampart-operators.config" "$PACKAGE/rampart-operators.config"
otool -L "$PACKAGE/libonnxruntime.dylib" >"$PACKAGE/dependencies.txt"
# Reject non-system dependencies rather than silently shipping a build-machine path.
if tail -n +3 "$PACKAGE/dependencies.txt" | grep -Ev '^[[:space:]]*(/usr/lib/|/System/Library/|$)' >/dev/null; then
  echo 'Unexpected non-system runtime dependency; inspect package/dependencies.txt.' >&2
  exit 1
fi
{
  echo "onnxruntime_version=$VERSION"
  echo "source_sha256=$SOURCE_SHA"
  echo "model_sha256=$MODEL_SHA"
  echo 'platform=darwin-arm64'
  echo 'deployment_target=13.0'
  echo 'providers=CPUExecutionProvider'
  echo 'model_format=ONNX'
  echo 'graph_optimization=disabled'
  echo "runtime_sha256=$(shasum -a 256 "$PACKAGE/libonnxruntime.dylib" | cut -d ' ' -f 1)"
  echo "runtime_bytes=$(wc -c <"$PACKAGE/libonnxruntime.dylib" | tr -d ' ')"
} >"$PACKAGE/build-manifest.txt"
cat "$PACKAGE/build-manifest.txt"
echo "Package: $PACKAGE"
