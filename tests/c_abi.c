/* Actual shared-library ABI smoke test. Arguments optionally override build defaults. */
#include "defingerprinter.h"
#include <assert.h>
#include <pthread.h>
#include <stdio.h>
#include <string.h>

static defingerprinter_handle handle;
static void *protect_in_thread(void *unused) {
    (void)unused;
    const char *input = "Contact Alice Johnson at zqx_75@example.com";
    uint8_t output[DEFINGERPRINTER_OUTPUT_CAPACITY + 1];
    size_t written = 99;
    int32_t status = defingerprinter_protect(handle, (const uint8_t *)"en", 2,
        (const uint8_t *)input, strlen(input), output, sizeof(output), &written);
    assert(status == DEFINGERPRINTER_OK || status == DEFINGERPRINTER_DISABLED);
    if (status == DEFINGERPRINTER_OK) {
        assert(written <= DEFINGERPRINTER_OUTPUT_CAPACITY);
        output[written] = 0;
        assert(strstr((char *)output, "Alice") == NULL);
        assert(strstr((char *)output, "Johnson") == NULL);
        assert(strstr((char *)output, "zqx_75") == NULL);
        assert(strstr((char *)output, "[EMAIL_1]") != NULL);
    } else { assert(written == 0); }
    return NULL;
}
int main(int argc, char **argv) {
    assert(defingerprinter_abi_version() == DEFINGERPRINTER_ABI_VERSION);
    assert(argc == 1 || argc == 3);
    int32_t status = argc == 3
        ? defingerprinter_create((const uint8_t *)argv[1], strlen(argv[1]), (const uint8_t *)argv[2], strlen(argv[2]), &handle)
        : defingerprinter_create(NULL, 0, NULL, 0, &handle);
    assert(status == DEFINGERPRINTER_OK && handle != 0);
    size_t written = 99;
    /* Disabled calls do not inspect diagnostic pointers or require an output buffer. */
    assert(defingerprinter_protect(handle, NULL, 2, NULL, 100, NULL, 0, &written) == DEFINGERPRINTER_DISABLED);
    assert(written == 0);
    assert(defingerprinter_set_consent(handle, 2) == DEFINGERPRINTER_INVALID_ARGUMENT);
    assert(defingerprinter_set_consent(handle, 1) == DEFINGERPRINTER_OK);
    uint8_t output[DEFINGERPRINTER_OUTPUT_CAPACITY + 1];
    assert(defingerprinter_protect(handle, (const uint8_t *)"en", 2, (const uint8_t *)"Alice", 5,
        output, 1, &written) == DEFINGERPRINTER_BUFFER_TOO_SMALL);
    assert(written == 0);
    const uint8_t invalid_utf8[] = {0xff};
    assert(defingerprinter_protect(handle, (const uint8_t *)"en", 2, invalid_utf8, 1,
        output, sizeof(output), &written) == DEFINGERPRINTER_UNSUPPORTED_INPUT);
    assert(written == 0);
    const char *unsupported = "١٢٣٤٥٦٧٨٩";
    assert(defingerprinter_protect(handle, (const uint8_t *)"en", 2, (const uint8_t *)unsupported, strlen(unsupported),
        output, sizeof(output), &written) == DEFINGERPRINTER_UNSUPPORTED_INPUT);
    protect_in_thread(NULL);
    pthread_t threads[4];
    for (size_t i = 0; i < 4; ++i) assert(pthread_create(&threads[i], NULL, protect_in_thread, NULL) == 0);
    assert(defingerprinter_set_consent(handle, 0) == DEFINGERPRINTER_OK);
    for (size_t i = 0; i < 4; ++i) assert(pthread_join(threads[i], NULL) == 0);
    assert(defingerprinter_protect(handle, NULL, 2, NULL, 100, NULL, 0, &written) == DEFINGERPRINTER_DISABLED);
    assert(written == 0);
    assert(defingerprinter_set_consent(handle, 1) == DEFINGERPRINTER_OK);
    protect_in_thread(NULL);
    assert(defingerprinter_destroy(handle) == DEFINGERPRINTER_OK);
    assert(defingerprinter_destroy(handle) == DEFINGERPRINTER_INVALID_HANDLE);
    assert(defingerprinter_protect(handle, NULL, 2, NULL, 100, NULL, 0, &written) == DEFINGERPRINTER_INVALID_HANDLE);
    assert(written == 0);
    puts("Defingerprinter C ABI: consent, UTF-8, redaction, concurrency and lifetime checks passed");
    return 0;
}
