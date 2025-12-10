# Nixification of Rust Build Scripts using Hardcoded Nix Store Paths (Interim Solution)

This document outlines a strategy for integrating C/C++ dependencies into Rust projects within a Nix environment. The immediate goal is to enable `build.rs` scripts to successfully compile and link against system libraries by hardcoding Nix store paths. This approach is a stepping stone towards a more automated solution using Rust procedural macros.

## Methodology

### 1. Identify and List External C/C++ Libraries

The first step is to identify all C/C++ libraries that the Rust crate's `build.rs` script needs to link against. This often includes libraries for features like compression (Snappy, LZ4, Zstd, Zlib, Bzip2), database systems (RocksDB), or specific system functionalities (io-uring).

### 2. Obtain Nix Store Paths for Required Libraries

For each identified library, use Nix commands to determine its exact store path (including development headers and library paths). This can typically be done in a `nix develop` shell by inspecting the `pkgs` object or using `nix eval`.

**Example Commands for Obtaining Paths:**

*   **For `glibc` development headers:**
    ```bash
    nix eval --raw --impure --expr 'pkgs.glibc.dev'
    ```
    (This would give a path like `/nix/store/gi4cz4ir3zlwhf1azqfgxqdnczfrwsr7-glibc-2.40-66-dev`)
*   **For `gcc` (wrapper) path:**
    ```bash
    nix eval --raw --impure --expr 'pkgs.gcc'
    ```
    (This would give a path like `/nix/store/vr15iyyykg9zai6fpgvhcgyw7gckl78w-gcc-wrapper-14.3.0`)
    To get the GCC version for C++ headers:
    ```bash
    nix eval --raw --impure --expr 'pkgs.lib.getVersion pkgs.gcc'
    ```
*   **For other library includes (e.g., `zlib`, `bzip2`):**
    ```bash
    nix eval --raw --impure --expr 'pkgs.zlib.dev'
    nix eval --raw --impure --expr 'pkgs.bzip2.dev'
    ```
    Append `/include` to these paths to get the correct include directories.

### 3. Hardcode Nix Store Paths into `build.rs`

Once the absolute Nix store paths are obtained, hardcode them directly into the `build.rs` file as Rust string literals. These variables should be placed at the top of the file or within a dedicated `struct` for easy management.

**Example `build.rs` Snippet:**

```rust
// librocksdb-sys/build.rs

#[derive(Debug, Clone)]
struct NixPaths {
    glibc_dev: String,
    gcc_path: String,
    gcc_version: String,
    zlib_include: String,
    bzip2_include: String,
    // ... other paths
}

impl NixPaths {
    fn default_nix_paths() -> Self {
        println!("cargo:warning=Using hardcoded default Nix paths.");
        NixPaths {
            glibc_dev: "/nix/store/gi4cz4ir3zlwhf1azqfgxqdnczfrwsr7-glibc-2.40-66-dev".to_string(),
            gcc_path: "/nix/store/vr15iyyykg9zai6fpgvhcgyw7gckl78w-gcc-wrapper-14.3.0".to_string(),
            gcc_version: "14.3.0".to_string(), // Manually extracted
            zlib_include: "/nix/store/hqvsiah013yzb17b13fn18fpqk7m13cg-zlib-1.3.1-dev/include".to_string(),
            bzip2_include: "/nix/store/q1a3bjhg3b4plgb7fk7zis1gi09rbi1d-bzip2-1.0.8-dev/include".to_string(),
            // ... initialize other paths
        }
    }
}
```

### 4. Conditionally Use Hardcoded Values in `build.rs`

The `build.rs` script should be written such that it *prefers* to read environment variables (e.g., `CC`, `CXX`, `CFLAGS`, `CXXFLAGS`, `CPATH`) set by a `nix develop` shell. However, if these environment variables are *not* set, it should fall back to using the hardcoded Nix store paths.

This allows the project to be built both within a Nix development shell (where environment variables are dynamically provided) and in environments where these variables might not be automatically set (relying on the hardcoded paths for a specific Nix store version).

**Example `build.rs` Logic:**

```rust
// librocksdb-sys/build.rs

fn main() {
    // ... (previous setup)

    let nix_paths = NixPaths::default_nix_paths();

    // Example: Configuring cc-rs build for C++ files
    let mut config = cc::Build::new();
    config.cpp(true); // Compile as C++

    // If CC/CXX environment variables are not set, cc-rs will use system defaults.
    // However, for Nix environments, we ensure the correct compiler is on PATH.

    // Add include paths using hardcoded Nix paths if not already covered by env vars or system defaults
    // It is crucial *not* to aggressively unset or overwrite CFLAGS/CXXFLAGS if they are coming from nix develop.
    // Rely on Nix's environment setup as much as possible.

    // Example for a specific include path:
    // config.include(&nix_paths.zlib_include);
    // config.include(&nix_paths.bzip2_include);

    // This section would contain the actual compilation of C/C++ sources
    // config.file("path/to/source.cc");
    // config.compile("my_library.a");

    // For bindgen, ensure LIBCLANG_PATH and LLVM_CONFIG are set
    // env::set_var("LIBCLANG_PATH", &nix_paths.libclang_path);
    // env::set_var("LLVM_CONFIG", &nix_paths.llvm_config);
    // env::set_var("LIBCLANG_FLAGS", &format!("--sysroot={}", nix_paths.glibc_dev));
    // ... (bindgen invocation) ...
}
```

### 5. Future Automation with Procedural Macros

The current hardcoding is an interim solution. The long-term goal is to replace this manual extraction and hardcoding with Rust procedural macros. These macros would be responsible for:

*   **Dynamically querying Nix:** At compile time, the proc macros would execute Nix commands (e.g., `nix eval`) to automatically discover the required store paths for dependencies.
*   **Generating Rust code:** The macros would then generate the necessary Rust code that injects these dynamically discovered paths into the `cc::Build` configuration and `bindgen` settings.

This would eliminate the need for manual path updates and ensure that the `build.rs` script always uses the correct dependency paths for the current Nix environment.

## Considerations

*   **Version Pinning:** When hardcoding paths, ensure they correspond to specific, stable versions of Nix packages to avoid breakage if the underlying Nix store changes.
*   **Environment Variables vs. Build Script:** Prioritize reading environment variables in `build.rs` when running within a `nix develop` shell. Only fall back to hardcoded paths if the relevant environment variables are not found.
*   **Verbose Logging:** Utilize verbose logging (`CARGO_CFG_TARGET_VERBOSE=1`) during the debugging phase to inspect the exact compiler commands and include paths used.
*   **Minimal Test Cases:** Create small, isolated test crates (like `test-cc-rs`) to reproduce and debug specific issues (e.g., `stdlib.h` not found) without the complexity of the full project.
*   **Workspace Management:** Be mindful of Cargo workspace configurations. For isolated tests, consider adding an empty `[workspace]` table to the test crate's `Cargo.toml` to prevent it from being treated as part of a larger workspace.
