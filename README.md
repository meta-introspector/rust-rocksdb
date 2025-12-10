# rust-rocksdb

[![RocksDB build](https://github.com/rust-rocksdb/rust-rocksdb/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/rust-rocksdb/rust-rocksdb/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/rocksdb.svg)](https://crates.io/crates/rocksdb)
[![documentation](https://img.shields.io/docsrs/rocksdb/latest)](https://docs.rs/rocksdb)
[![license](https://img.shields.io/crates/l/rocksdb.svg)](https://github.com/rust-rocksdb/rust-rocksdb/blob/master/LICENSE)
[![Gitter chat](https://badges.gitter.im/rust-rocksdb/gitter.svg)](https://gitter.im/rust-rocksdb/lobby)
![rust 1.85.0 required](https://img.shields.io/badge/rust-1.85.0-blue.svg?label=MSRV)

![GitHub commits (since latest release)](https://img.shields.io/github/commits-since/rust-rocksdb/rust-rocksdb/latest.svg)

## Requirements

- Clang and LLVM

## Contributing

Feedback and pull requests are welcome! If a particular feature of RocksDB is
important to you, please let me know by opening an issue, and I'll
prioritize it.

## Usage

This binding is statically linked with a specific version of RocksDB. If you
want to build it yourself, make sure you've also cloned the RocksDB and
compression submodules:

```shell
git submodule update --init --recursive
```

## Compression Support

By default, support for [Snappy](https://github.com/google/snappy),
[LZ4](https://github.com/lz4/lz4), [Zstd](https://github.com/facebook/zstd),
[Zlib](https://zlib.net), and [Bzip2](http://www.bzip.org) compression
is enabled through crate features. If support for all of these compression
algorithms is not needed, default features can be disabled and specific
compression algorithms can be enabled. For example, to enable only LZ4
compression support, make these changes to your Cargo.toml:

```toml
[dependencies.rocksdb]
default-features = false
features = ["lz4"]
```

## Multithreaded ColumnFamily alternation

RocksDB allows column families to be created and dropped
from multiple threads concurrently, but this crate doesn't allow it by default
for compatibility. If you need to modify column families concurrently, enable
the crate feature `multi-threaded-cf`, which makes this binding's
data structures use `RwLock` by default. Alternatively, you can directly create
`DBWithThreadMode<MultiThreaded>` without enabling the crate feature.

## Switch between /MT or /MD run time library (Only for Windows)

The feature `mt_static` will request the library to be built with [/MT](https://learn.microsoft.com/en-us/cpp/build/reference/md-mt-ld-use-run-time-library?view=msvc-170)
flag, which results in library using the static version of the run-time library.
*This can be useful in case there's a conflict in the dependency tree between different
run-time versions.*

## Switch between static and dynamic linking for bindgen (features `bindgen-static` and `bindgen-runtime`)

The feature `bindgen-runtime` will enable the `runtime` feature of bindgen, which dynamically
links to libclang. This is suitable for most platforms, and is enabled by default.

The feature `bindgen-static` will enable the `static` feature of bindgen, which statically
links to libclang. This is suitable for musllinux platforms, such as Alpine linux.
To build on Alpine linux for example, make these changes to your Cargo.toml:

```toml
[dependencies.rocksdb]
default-features = false
features = ["bindgen-static", "snappy", "lz4", "zstd", "zlib", "bzip2"]
```

Notice that `runtime` and `static` features are mutually exclusive, and won't compile if both are enabled.

## Switch between static and dynamic linking for bindgen (features `bindgen-static` and `bindgen-runtime`)

The feature `bindgen-runtime` will enable the `runtime` feature of bindgen, which dynamically
links to libclang. This is suitable for most platforms, and is enabled by default.

The feature `bindgen-static` will enable the `static` feature of bindgen, which statically
links to libclang. This is suitable for musllinux platforms, such as Alpine linux.
To build on Alpine linux for example, make these changes to your Cargo.toml:

```toml
[dependencies.rocksdb]
default-features = false
features = ["bindgen-static", "snappy", "lz4", "zstd", "zlib", "bzip2"]
```

Notice that `runtime` and `static` features are mutually exclusive, and won't compile if both are enabled.

## Nix Build Integration for C/C++ Dependencies

The `test-cc-rs` crate serves as a crucial integration test to verify the correct setup of C/C++ dependencies within a Nix environment. It mimics the build process for `librocksdb-sys`, ensuring that explicit Nix store paths for compilers, system headers, and external libraries (like Snappy, LZ4, Zstd, Zlib, and Bzip2) are correctly managed by `build.rs`.

Key aspects tested and demonstrated by `test-cc-rs` include:

*   **Explicit Path Management:** `build.rs` leverages hardcoded Nix store paths for `glibc` development headers, GCC compilers, and external library includes (e.g., `lz4_include`, `zstd_include`).
*   **Compiler Environment Configuration:** Environment variables like `CC`, `CXX`, `CPATH`, and `CXXFLAGS` are explicitly set within `build.rs` to guide the `cc-rs` crate and underlying C/C++ compilers, resolving common `stdlib.h` and header lookup issues in Nix.
*   **Dynamic Linking:** External libraries such as LZ4 and Zstd are configured for dynamic linking (`dylib`) to align with how these libraries are typically provided in Nix, addressing linker errors for static library lookups.
*   **Custom Header Generation:** For certain complex dependencies like Snappy, `test-cc-rs` demonstrates the on-the-fly generation of necessary header files (e.g., `snappy-stubs-public.h` from `snappy-stubs-public.h.in`) during the build process to ensure all required definitions are available.

This integration test ensures that `librocksdb-sys` can be reliably built and linked against its C/C++ dependencies in a reproducible Nix build environment.

## LTO
Enable the `lto` feature to enable link-time optimization. It will compile rocksdb with `-flto` flag. This feature is disabled by default.

> [!IMPORTANT]
> You must use clang as `CC`. Eg. `CC=/usr/bin/clang CXX=/usr/bin/clang++`. Clang llvm version must be the same as the one used by rust compiler.
> On the rust side you should use `RUSTFLAGS="-Clinker-plugin-lto -Clinker=clang -Clink-arg=-fuse-ld=lld"`.

Check the [Rust documentation](https://doc.rust-lang.org/rustc/linker-plugin-lto.html) for more information.
