# Integrating `librocksdb-sys` into a Nix Environment

This document outlines the process and key learnings from successfully building the `librocksdb-sys` crate in a Nix environment. The primary challenge was ensuring the C/C++ compiler (`g++`) and related build tools (`bindgen`, `cc-rs`) correctly located standard library headers and other dependencies from Nix store paths, often encountering "fatal error: stdlib.h: No such file or directory".

The solution emphasizes explicit control over compiler flags and environment variables within the `build.rs` script.

## 1. Problem Statement

Building Rust crates with C/C++ dependencies (like `librocksdb-sys`) in a Nix environment presents unique challenges due to Nix's strict dependency management and isolated build environments. Common issues encountered include:

*   **`stdlib.h` / `features.h` not found:** Even when `glibc` development headers are included in the build environment, `g++` may fail to locate fundamental C standard library headers, especially when invoked via `include_next` from C++ standard library headers.
*   **Environment variable interference:** `CXXFLAGS`, `CPATH`, and similar environment variables, if implicitly or explicitly set by a `flake.nix` `devShell` or other Nix mechanisms, can conflict with the explicit flags and paths managed by `build.rs` via `cc-rs` and `bindgen`.
*   **`--sysroot` flag complexities:** The `--sysroot` compiler flag, while generally useful for custom toolchains, can sometimes interact unexpectedly with `g++`'s internal search paths in a Nix context, leading to header discovery failures.

## 2. Solution Overview

The successful approach involves a combination of:
*   **Aggressive environment variable unsetting** in `build.rs`.
*   **Explicitly hardcoded (for now) Nix store paths** within `build.rs` for all critical dependencies (`glibc`, `gcc`, `zlib`, `bzip2`, `lz4`, `zstd`).
*   **Precise configuration of `cc-rs`** with explicit compiler paths and specific include flags (`-isystem`, `-I`), avoiding the `--sysroot` flag for `g++` compilation.
*   **Careful management of `flake.nix`** `devShell` to prevent environment variable pollution.

## 3. Changes to `librocksdb-sys/build.rs`

The `build.rs` script is central to configuring the C/C++ build process. It should be adapted as follows:

### 3.1. `NixPaths` Structure and Initialization

Introduce a `NixPaths` struct to centralize all hardcoded Nix store paths. These paths should be manually extracted (e.g., using `nix eval --raw --impure --expr 'pkgs.package_name.dev'` or by inspecting `g++ -Wp,-v -x c++ -` output within a `nix develop` shell).

```rust
use std::env;

#[derive(Debug, Clone)]
struct NixPaths {
    // Basic C/C++ toolchain
    glibc_dev: String,          // Path to glibc development headers
    gcc_path: String,           // Path to the GCC derivation (containing bin/g++)
    gcc_cpp_include: String,    // Path to GCC's C++ standard library headers

    // External library includes (examples)
    zlib_include: String,
    bzip2_include: String,
    lz4_include: String,
    zstd_include: String,

    // For bindgen (if used)
    llvm_config: String,
    libclang_path: String,
    llvm_config_path: String,
}

impl NixPaths {
    fn default_nix_paths() -> Self {
        println!("cargo:warning=Using hardcoded default Nix paths.");
        NixPaths {
            // --- Basic C/C++ Toolchain Paths ---
            // Example: nix eval --raw --impure --expr 'pkgs.glibc.dev'
            glibc_dev: "/nix/store/gi4cz4ir3zlwhf1azqfgxqdnczfrwsr7-glibc-2.40-66-dev".to_string(),
            // Example: Actual GCC derivation path from 'g++ -Wp,-v' output
            gcc_path: "/nix/store/82kmz7r96navanrc2fgckh2bamiqrgsw-gcc-14.3.0".to_string(),
            // Example: Path to GCC's C++ headers
            gcc_cpp_include: "/nix/store/82kmz7r96navanrc2fgckh2bamiqrgsw-gcc-14.3.0/include/c++/14.3.0".to_string(),

            // --- External Library Include Paths ---
            // Example: nix eval --raw --impure --expr 'pkgs.zlib.dev'
            zlib_include: "/nix/store/hqvsiah013yzb17b13fn18fpqk7m13cg-zlib-1.3.1-dev/include".to_string(),
            // Example: nix eval --raw --impure --expr 'pkgs.bzip2.dev'
            bzip2_include: "/nix/store/q1a3bjhg3b4plgb7fk7zis1gi09rbi1d-bzip2-1.0.8-dev/include".to_string(),
            // Placeholder: Replace with actual LZ4 dev include path
            lz4_include: "/nix/store/somehash-lz4-1.9.4-dev/include".to_string(),
            // Placeholder: Replace with actual ZSTD dev include path
            zstd_include: "/nix/store/somehash-zstd-1.5.5-dev/include".to_string(),

            // --- LLVM/Clang Paths for Bindgen (examples from librocksdb-sys) ---
            // Example: nix eval --raw --impure --expr 'pkgs.llvmPackages_21.llvm.dev'
            llvm_config: "/nix/store/v9cr3iv7wnrkjy1s3z1fi7wpkl7sy4hx-llvm-21.1.2-dev/bin/llvm-config".to_string(),
            // Example: nix eval --raw --impure --expr 'pkgs.llvmPackages_21.libclang.lib'
            libclang_path: "/nix/store/sqlnjj8c3n3si3sjnadhdbcwgrk97g2w-clang-wrapper-21.1.2/lib".to_string(),
            // Example: nix eval --raw --impure --expr 'pkgs.llvmPackages_21.llvm'
            llvm_config_path: "/nix/store/b5bmnvk17mq8qm5b8bpi9fkyr5g2d2m4-llvm-21.1.2/lib".to_string(),
        }
    }
}
```

### 3.2. Aggressive Environment Variable Unsetting

At the very beginning of `main()`, aggressively unset any environment variables that might interfere with `cc-rs` or `bindgen`. This ensures a clean slate and forces reliance on explicitly provided paths.

```rust
fn main() {
    // Unset potentially interfering environment variables
    env::remove_var("CXX");
    env::remove_var("CXXFLAGS");
    env::remove_var("CPATH");
    env::remove_var("C_INCLUDE_PATH");
    env::remove_var("CPLUS_INCLUDE_PATH");
    // Add other relevant variables if they are found to interfere:
    // env::remove_var("CC");
    // env::remove_var("CFLAGS");
    // env::remove_var("LIBRARY_PATH");
    // env::remove_var("LD_LIBRARY_PATH");
    // env::remove_var("PROTOC");
    // env::remove_var("PROTOC_INCLUDE");
    // env::remove_var("BINDGEN_EXTRA_CLANG_ARGS");
    // env::remove_var("LLVM_CONFIG");
    // env::remove_var("LLVM_CONFIG_PATH");
    // env::remove_var("LIBCLANG_PATH");
    // env::remove_var("LIBCLANG_FLAGS");
    // env::remove_var("NIX_GLIBC_DEV");
    // env::remove_var("NIX_GCC_PATH");
    // env::remove_var("NIX_GCC_REAL_PATH");

    let nix_paths = NixPaths::default_nix_paths();
    // ... rest of main()
}
```

### 3.3. `cc::Build` Configuration for C/C++ Compilation

Configure `cc::Build` to explicitly use the Nix-provided `g++` and include paths. Crucially, avoid the `--sysroot` flag for `g++` if it causes issues; instead, rely on explicit `-isystem` and `-I` flags.

```rust
    let mut build = cc::Build::new();
    build.cpp(true); // Always compile as C++ for RocksDB

    // Explicitly set the compiler
    build.compiler(&format!("{}/bin/g++", nix_paths.gcc_path));

    // Explicitly add include paths
    // Use -isystem for Glibc C headers (standard system headers)
    build.flag(&format!("-isystem{}/include", nix_paths.glibc_dev));
    // Use -I for GCC's C++ standard library headers
    build.include(&nix_paths.gcc_cpp_include);

    // Conditional includes and defines for external libraries (mimicking librocksdb-sys)
    if cfg!(feature = "zlib") {
        build.define("ZLIB", Some("1"));
        build.include(&nix_paths.zlib_include);
        // Add linking flag if necessary for system Zlib
        println!("cargo:rustc-link-lib=static=z"); // Example linking
    }
    if cfg!(feature = "bzip2") {
        build.define("BZIP2", Some("1"));
        build.include(&nix_paths.bzip2_include);
        println!("cargo:rustc-link-lib=static=bz2"); // Example linking
    }
    if cfg!(feature = "lz4") {
        build.define("LZ4", Some("1"));
        build.include(&nix_paths.lz4_include);
        println!("cargo:rustc-link-lib=static=lz4"); // Example linking
    }
    if cfg!(feature = "zstd") {
        build.define("ZSTD", Some("1"));
        build.include(&nix_paths.zstd_include);
        println!("cargo:rustc-link-lib=static=zstd"); // Example linking
    }

    // Add other RocksDB-specific compilation flags, defines, and source files here
    // ... (e.g., from original librocksdb-sys/build.rs)

    build.compile("librocksdb.a"); // Compile the RocksDB static library
```

### 3.4. `bindgen` Configuration

Ensure `bindgen` correctly finds `libclang` and system headers. It's often safer to explicitly set `LIBCLANG_PATH` and `LLVM_CONFIG`. For `bindgen`'s internal clang, the `--sysroot` flag is typically more effective than for `g++` itself.

```rust
fn bindgen_rocksdb(nix_paths: &NixPaths) {
    env::set_var("LIBCLANG_PATH", &nix_paths.libclang_path);
    env::set_var("LLVM_CONFIG_PATH", &nix_paths.llvm_config_path);
    env::set_var("LLVM_CONFIG", &nix_paths.llvm_config);
    
    // For bindgen, --sysroot for its internal clang is often reliable.
    env::set_var("LIBCLANG_FLAGS", &format!("--sysroot={}", nix_paths.glibc_dev));
    
    let bindings = bindgen::Builder::default()
        .header(rocksdb_include_dir() + "/rocksdb/c.h") // Assuming rocksdb_include_dir() is defined
        // ... other bindgen configurations
        .generate()
        .expect("unable to generate rocksdb bindings");
    // ... write bindings to file
}
```

## 4. Changes to `librocksdb-sys/Cargo.toml`

Update `Cargo.toml` to include the `build-dependencies` for the sys-crates corresponding to the external C libraries.

```toml
[build-dependencies]
cc = { version = "1.0", features = ["parallel"] }
bindgen = { version = "0.72", default-features = false }
pkg-config = { version = "0.3", optional = true } # Needed for io-uring feature
libz-sys = { version = "1.1", default-features = false, optional = true }
bzip2-sys = { version = "0.1", default-features = false, optional = true }
lz4-sys = { version = "1.10", optional = true }
zstd-sys = { version = "2.0", default-features = false, features = ["legacy", "zdict_builder"], optional = true }

# Ensure corresponding features are added to your [features] section
[features]
# ... existing features
zlib = ["libz-sys"]
bzip2 = ["bzip2-sys"]
lz4 = ["lz4-sys"]
zstd = ["zstd-sys"]
```
*(Note: `optional = true` and corresponding `[features]` entries are crucial for allowing these to be toggled by the user).*

## 5. Changes to `flake.nix` (`devShell`)

To minimize environment variable interference, configure the `devShell` in `flake.nix` to avoid setting `CXXFLAGS`, `CFLAGS`, `CPATH`, etc., in its `shellHook` if `build.rs` is taking explicit control. If they must be set for other reasons, comment them out or manage them carefully.

```nix
# flake.nix snippet
# ...
devShells.default = pkgs.mkShell {
  packages = with pkgs;
    [ # ... existing packages
      glibc.dev     # Provides C standard library headers
      gcc           # Provides g++ compiler
      clang
      llvmPackages.libclang
      llvmPackages.llvm
      zlib.dev      # Zlib development headers
      bzip2.dev     # Bzip2 development headers
      lz4.dev       # LZ4 development headers
      zstd.dev      # ZSTD development headers
      pkg-config    # For pkg-config
    ];

  shellHook = ''
    # Environment variables needed by build.rs (example)
    export LIBCLANG_PATH="/nix/store/10mkp77lmqz8x2awd8hzv6pf7f7rkf6d-clang-19.1.7-lib/lib";
    # ... other LIBCLANG/LLVM related exports

    # CFLAGS and CXXFLAGS should generally NOT be exported if build.rs takes full control.
    # If they are exported for other tools, consider commenting them out or ensuring they
    # don't conflict.
    # export CFLAGS="-O2 -g";
    # export CXXFLAGS="-O2 -g -isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.gcc}/include/c++/${pkgs.gcc.version}"; # Commented out as it interfered with build.rs explicit compiler flags.

    # ... other shellHook exports
  '';
};
# ...
```

## 6. Key Learnings and Recommendations

*   **Explicit Control in `build.rs` is Paramount:** In Nix, relying on implicit environment variables for C/C++ compilation is unreliable. `build.rs` should aggressively unset interfering environment variables and explicitly configure `cc-rs` and `bindgen` with precise Nix store paths.
*   **`--sysroot` vs. `-isystem`:** For `g++` compilation in this context, removing the `--sysroot` flag and instead using explicit `-isystem` and `-I` flags for each required include path proved more robust for resolving standard library headers. The interaction of `--sysroot` with `g++`'s internal C++ standard library includes (e.g., `cstdlib`'s `include_next`) can be problematic.
*   **Accurate Nix Store Paths:** Always verify the exact Nix store paths for `glibc.dev`, `gcc`, `llvm`, `clang`, and other dependencies using `nix eval` or by inspecting compiler verbose output. Derivations and their internal structures can vary.
*   **Future Work (Proc-Macros):** The hardcoded paths are an interim solution. The long-term goal is to replace manual extraction and hardcoding with Rust procedural macros that dynamically query Nix at compile time to discover required store paths. This would eliminate manual updates and improve maintainability.

By following these guidelines, `librocksdb-sys` can be successfully integrated and built within a Nix environment, providing a robust and reproducible build process.
