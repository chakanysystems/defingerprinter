#include "defingerprinter.h"
#include <array>
#include <iostream>
#include <string>

int main() {
    if (defingerprinter_abi_version() != DEFINGERPRINTER_ABI_VERSION)
        return 1;

    defingerprinter_handle handle;
    auto status = defingerprinter_create(nullptr, 0, nullptr, 0, &handle);
    if (status != DEFINGERPRINTER_OK) {
        std::cerr << "create failed: " << status << '\n';
        return 1;
    }

    const std::string language = "en";
    const std::string input = "Contact alice@example.com";
    std::array<uint8_t, DEFINGERPRINTER_OUTPUT_CAPACITY> output;
    size_t written;
    status = defingerprinter_redact(handle,
        reinterpret_cast<const uint8_t*>(language.data()), language.size(),
        reinterpret_cast<const uint8_t*>(input.data()), input.size(),
        output.data(), output.size(), &written);
    defingerprinter_destroy(handle);

    if (status != DEFINGERPRINTER_OK) {
        std::cerr << "redact failed: " << status << '\n';
        return 1;
    }
    std::cout << std::string(reinterpret_cast<const char*>(output.data()), written) << '\n';
}
