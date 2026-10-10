import ctypes as c
import sys

# C ABI declarations.
lib = c.CDLL(sys.argv[1])
Handle = c.c_uint64
lib.defingerprinter_abi_version.argtypes = []
lib.defingerprinter_abi_version.restype = c.c_uint32
lib.defingerprinter_create.argtypes = [
    c.c_char_p, c.c_size_t, c.c_char_p, c.c_size_t, c.POINTER(Handle)
]
lib.defingerprinter_create.restype = c.c_int32
lib.defingerprinter_protect.argtypes = [
    Handle, c.c_char_p, c.c_size_t, c.c_char_p, c.c_size_t,
    c.c_void_p, c.c_size_t, c.POINTER(c.c_size_t)
]
lib.defingerprinter_protect.restype = c.c_int32
lib.defingerprinter_destroy.argtypes = [Handle]
lib.defingerprinter_destroy.restype = c.c_int32


def check(status):
    if status != 0:
        raise RuntimeError(f"Defingerprinter status {status}")


# Create, protect, destroy.
if lib.defingerprinter_abi_version() != 2:
    raise RuntimeError("Expected Defingerprinter ABI 2")
handle = Handle()
check(lib.defingerprinter_create(None, 0, None, 0, c.byref(handle)))
try:
    language = b"en"
    text = "Contact alice@example.com".encode("utf-8")
    output = c.create_string_buffer(4096)
    written = c.c_size_t()
    check(lib.defingerprinter_protect(
        handle, language, len(language), text, len(text),
        output, len(output), c.byref(written)
    ))
    print(output.raw[:written.value].decode("utf-8"))  # Contact [EMAIL_1]
finally:
    check(lib.defingerprinter_destroy(handle))
