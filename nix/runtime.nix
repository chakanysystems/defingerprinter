{ lib, stdenv, buildPackages, fetchurl, runCommand, cmake, ninja, unzip, python3, darwinMinVersionHook }:
let
  pins = import ./dependencies.nix;
  python = python3.withPackages (p: [ p.flatbuffers p.numpy ]);
  # Nix fetches before the sandboxed build. CMake must never fetch dependencies.
  names = [ "abseil_cpp" "date" "dlpack" "eigen" "flatbuffers" "mp11" "json" "microsoft_gsl" "onnx" "protobuf" "pytorch_cpuinfo" "re2" "safeint" "kleidiai" ];
  sources = lib.genAttrs names (name:
    let archive = fetchurl (pins.${name} // { name = "ort-${name}-source"; });
    in runCommand "ort-${name}-unpacked" { nativeBuildInputs = [ unzip ]; } ''
      mkdir unpack "$out"
      cd unpack
      case ${lib.escapeShellArg pins.${name}.url} in
        *.zip) unzip -q ${archive} ;;
        *) tar xf ${archive} ;;
      esac
      cp -R */. "$out/"
    '');
  # Avoid runtime-closure references introduced by dependency __FILE__ strings.
  sourceMaps = lib.concatStringsSep " " (lib.mapAttrsToList
    (name: source: "-ffile-prefix-map=${source}=/defingerprinter-build/${name}") sources);
in stdenv.mkDerivation {
  pname = "defingerprinter-onnxruntime";
  version = "1.24.2";
  src = fetchurl {
    url = "https://codeload.github.com/microsoft/onnxruntime/tar.gz/refs/tags/v1.24.2";
    sha256 = "e4f44579f0bdd06d1cc7abdb2a6740dc06ac10699d502c0f8d9576203550bd4d";
  };
  unpackCmd = "tar xzf $curSrc";
  nativeBuildInputs = [ cmake ninja python ];
  buildInputs = lib.optionals stdenv.hostPlatform.isDarwin [ (darwinMinVersionHook "13.3") ];
  NIX_CFLAGS_COMPILE = sourceMaps;
  disallowedReferences = builtins.attrValues sources;
  cmakeBuildType = "MinSizeRel";
  cmakeDir = "../cmake";
  preConfigure = ''
    ${python.interpreter} tools/ci_build/reduce_op_kernels.py ${./rampart-operators.config} --cmake_build_dir build
  '';
  cmakeFlags = [
    "--compile-no-warning-as-error"
    # No C++ modules are used. clang-scan-deps bypasses Nix's compiler-wrapper include flags.
    "-DCMAKE_CXX_SCAN_FOR_MODULES=OFF"
    "-DCMAKE_POLICY_VERSION_MINIMUM=3.5"
    "-DFETCHCONTENT_FULLY_DISCONNECTED=ON"
    "-DFETCHCONTENT_TRY_FIND_PACKAGE_MODE=NEVER"
    "-Donnxruntime_BUILD_SHARED_LIB=ON"
    "-Donnxruntime_BUILD_UNIT_TESTS=OFF"
    "-Donnxruntime_REDUCED_OPS_BUILD=ON"
    "-Donnxruntime_DISABLE_ML_OPS=ON"
    "-Donnxruntime_DISABLE_RTTI=ON"
    "-Donnxruntime_ENABLE_LTO=ON"
    "-Donnxruntime_ENABLE_TRAINING=OFF"
    "-Donnxruntime_ENABLE_TRAINING_OPS=OFF"
    "-Donnxruntime_USE_COREML=OFF"
    "-Donnxruntime_USE_CUDA=OFF"
    "-Donnxruntime_USE_OPENVINO=OFF"
    "-Donnxruntime_USE_DNNL=OFF"
    "-Donnxruntime_USE_XNNPACK=OFF"
    "-Donnxruntime_ENABLE_PYTHON=OFF"
    "-Donnxruntime_USE_FULL_PROTOBUF=OFF"
    "-DONNX_CUSTOM_PROTOC_EXECUTABLE=${lib.getExe buildPackages.protobuf_21}"
    "-DFETCHCONTENT_SOURCE_DIR_NLOHMANN_JSON=${sources.json}"
    "-DFETCHCONTENT_SOURCE_DIR_GSL=${sources.microsoft_gsl}"
    "-DFETCHCONTENT_SOURCE_DIR_EIGEN3=${sources.eigen}"
    "-DCMAKE_C_COMPILER_AR=${stdenv.cc.bintools}/bin/ar"
    "-DCMAKE_CXX_COMPILER_AR=${stdenv.cc.bintools}/bin/ar"
    "-DCMAKE_C_COMPILER_RANLIB=${stdenv.cc.bintools}/bin/ranlib"
    "-DCMAKE_CXX_COMPILER_RANLIB=${stdenv.cc.bintools}/bin/ranlib"
  ] ++ lib.mapAttrsToList (name: src: "-DFETCHCONTENT_SOURCE_DIR_${lib.toUpper name}=${src}") sources;
  enableParallelBuilding = true;
  doCheck = false; # The package's C ABI smoke check loads the actual pinned graph.
  postInstall = ''
    # Upstream prefixes absolute GNUInstallDirs paths a second time in its .pc template.
    substituteInPlace "$out/lib/pkgconfig/libonnxruntime.pc" \
      --replace-fail "\''${prefix}/$out" "\''${prefix}"
    # Protobuf is statically linked; its incidental CMake export points at build sources.
    rm -rf "$out/lib/cmake/protobuf"
    mkdir -p "$out/share/licenses/onnxruntime" "$out/share/defingerprinter"
    cp ../LICENSE ../ThirdPartyNotices.txt "$out/share/licenses/onnxruntime/"
    cp ${./rampart-operators.config} "$out/share/defingerprinter/rampart-operators.config"
  '';
  meta = {
    description = "CPU-only ONNX Runtime reduced to the pinned Rampart q4 graph";
    license = lib.licenses.mit;
    platforms = [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ];
  };
}
