{ lib, fetchurl, runCommand }:
let
  revision = "b1993e4e68b082835b80ffc65acc03325ea2e501";
  hashes = {
    "config.json" = "003b84bbcd489f5e782fe5cad8f3249c3653ec880089abb1ccc398a0d895e3e6";
    "tokenizer.json" = "98ade711428b42a1b5343c403a73344535e92de8e19359cdb567ef34da210259";
    "tokenizer_config.json" = "0088a6f8bcdd4014184fb068b83ebb12896a9db2bb269a71f73de83fef24bceb";
    "onnx/model_q4.onnx" = "9f27d24949b0581701071ea5ef522d77ccd3f50c525cc91eac4d265b0fc2afe5";
  };
  files = lib.mapAttrs (name: sha256: fetchurl {
    url = "https://huggingface.co/NationalDesignStudio/rampart/resolve/${revision}/${name}";
    inherit sha256;
    name = builtins.baseNameOf name;
  }) hashes;
in runCommand "defingerprinter-rampart-model-${builtins.substring 0 12 revision}" {
  meta = {
    description = "Original pinned Rampart quantized ONNX model and tokenizer";
    license = lib.licenses.cc-by-40;
  };
  passthru = { inherit revision hashes; };
} (''
  mkdir -p "$out/onnx" "$out/licenses"
  cp ${../ATTRIBUTION.md} "$out/ATTRIBUTION.md"
  cp ${../licenses/CC-BY-4.0.txt} "$out/licenses/CC-BY-4.0.txt"
'' + lib.concatStringsSep "\n" (lib.mapAttrsToList (name: file: ''
  cp ${file} "$out/${name}"
'') files))
