// snappy-stubs-public.h - minimal version for test-cc-rs
#ifndef THIRD_PARTY_SNAPPY_OPENSOURCE_SNAPPY_STUBS_PUBLIC_H_
#define THIRD_PARTY_SNAPPY_OPENSOURCE_SNAPPY_STUBS_PUBLIC_H_

#include <cstddef> // For size_t
#include <sys/uio.h> // For struct iovec on Linux

// Minimal definitions for compilation
#define SNAPPY_MAJOR 1
#define SNAPPY_MINOR 2
#define SNAPPY_PATCHLEVEL 2
#define SNAPPY_VERSION ((SNAPPY_MAJOR << 16) | (SNAPPY_MINOR << 8) | SNAPPY_PATCHLEVEL)

namespace snappy {
    // Define iovec if sys/uio.h didn't provide it (though it should on Linux)
    // This block is based on the logic in snappy-stubs-public.h.in
    #ifndef HAVE_SYS_UIO_H
    struct iovec {
      void* iov_base;
      size_t iov_len;
    };
    #endif
}

#endif  // THIRD_PARTY_SNAPPY_OPENSOURCE_SNAPPY_STUBS_PUBLIC_H_