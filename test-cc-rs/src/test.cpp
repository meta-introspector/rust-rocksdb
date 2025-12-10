#include <cstdlib>
#include "snappy.h" // Include snappy header

int main() {
    // Test a simple Snappy operation
    const char* input = "Hello Snappy!";
    size_t input_length = 13; // strlen("Hello Snappy!")
    size_t compressed_length = snappy::MaxCompressedLength(input_length);
    char* compressed = new char[compressed_length];

    snappy::RawCompress(input, input_length, compressed, &compressed_length);
    // RawCompress does not return a boolean for success,
    // so we can't check for failure in the same way.
    // For a simple test, we assume it succeeds and then clean up.

    // Optionally decompress and verify, but for now just testing compilation and linking
    delete[] compressed;
    return 0;
}