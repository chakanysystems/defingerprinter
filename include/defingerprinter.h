/* Chakany Systems. Stable Defingerprinter C ABI, version 1. */
#ifndef DEFINGERPRINTER_H
#define DEFINGERPRINTER_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef uint64_t defingerprinter_handle;
#define DEFINGERPRINTER_ABI_VERSION 1u
#define DEFINGERPRINTER_MAX_INPUT_BYTES 16384u
#define DEFINGERPRINTER_OUTPUT_CAPACITY 4096u

enum defingerprinter_status {
    DEFINGERPRINTER_OK = 0,
    DEFINGERPRINTER_INVALID_ARGUMENT = 1,
    DEFINGERPRINTER_INVALID_HANDLE = 2,
    DEFINGERPRINTER_DISABLED = 3,
    DEFINGERPRINTER_UNSUPPORTED_INPUT = 4,
    DEFINGERPRINTER_ARTIFACT_ERROR = 5,
    DEFINGERPRINTER_RUNTIME_ERROR = 6,
    DEFINGERPRINTER_INFERENCE_ERROR = 7,
    DEFINGERPRINTER_BUFFER_TOO_SMALL = 8,
    DEFINGERPRINTER_INTERNAL_ERROR = 9
};

uint32_t defingerprinter_abi_version(void);
/* UTF-8 absolute paths, each <=4096 bytes. Pass NULL,0 for BOTH paths to use
   immutable Nix build defaults. Starts disabled; does not load the model.
   out_handle must be writable. It is zero on error. */
int32_t defingerprinter_create(const uint8_t *model_dir, size_t model_dir_len,
    const uint8_t *runtime, size_t runtime_len, defingerprinter_handle *out_handle);
/* enabled is exactly 0 or 1. Revocation waits for in-flight work and drops the session. */
int32_t defingerprinter_set_consent(defingerprinter_handle handle, int32_t enabled);
/* UTF-8 slices. Language is en/es/fr/de/it/pt/nl. Caller owns the output buffer
   (>=OUTPUT_CAPACITY bytes) and written. Output is NOT NUL-terminated.
   written is zero on failure; ignore the buffer on failure. No text is retained
   for retries. Consent is checked before reading language/input/output.
   All non-null pointers used by a call must reference valid memory for the stated
   length, and written must be valid and disjoint from the slices. Each concurrent
   call must use distinct output and written storage. Model execution/publication
   are serialized per handle; different handles can run concurrently. */
int32_t defingerprinter_protect(defingerprinter_handle handle,
    const uint8_t *language, size_t language_len, const uint8_t *input, size_t input_len,
    uint8_t *output, size_t output_capacity, size_t *written);
/* Safe for concurrent callers. Stale and duplicate handles return INVALID_HANDLE.
   Completes outstanding work before returning. Do not unload the library while
   any call or live handle exists. */
int32_t defingerprinter_destroy(defingerprinter_handle handle);

#ifdef __cplusplus
}
#endif
#endif
