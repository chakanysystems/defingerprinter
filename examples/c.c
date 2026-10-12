#include "defingerprinter.h"
#include <stdio.h>

int main(void) {
    if (defingerprinter_abi_version() != DEFINGERPRINTER_ABI_VERSION)
        return 1;

    defingerprinter_handle handle;
    int32_t status = defingerprinter_create(NULL, 0, NULL, 0, &handle);
    if (status != DEFINGERPRINTER_OK) {
        fprintf(stderr, "create failed: %d\n", (int)status);
        return 1;
    }

    const uint8_t language[] = "en";
    const uint8_t input[] = "Contact alice@example.com";
    uint8_t output[DEFINGERPRINTER_OUTPUT_CAPACITY];
    size_t written;
    status = defingerprinter_redact(handle,
        language, sizeof language - 1, input, sizeof input - 1,
        output, sizeof output, &written);
    defingerprinter_destroy(handle);

    if (status != DEFINGERPRINTER_OK) {
        fprintf(stderr, "redact failed: %d\n", (int)status);
        return 1;
    }
    fwrite(output, 1, written, stdout);
    putchar('\n');
    return 0;
}
