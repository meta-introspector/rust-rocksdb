use std::path::Path;
use std::{env, fs, path::PathBuf, process::Command};
use rust_nix_bootstrap::{NixPathsProvider, HardcodedNixPaths};
// Removed serde imports as dynamic loading/saving is temporarily bypassed

//const CACHE_FILE_NAME: &str = ".nix_paths_cache.json"; // Still here for context, not actively used right now

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
    lz4_lib: String, // New field for lz4 library path
    zstd_include: String,
    zstd_lib: String, // New field for zstd library path

    // For bindgen (if used)
    llvm_config: String,
    libclang_path: String,
    llvm_config_path: String,
}

// Helper function to get Nix path from environment variable or hardcoded fallback
fn get_nix_path_from_env(env_var: &str, hardcoded_path_from_provider: &'static str) -> String {
    let env_val = env::var(env_var);

    match env_val {
        Ok(path_from_env) => {
            // Compare with hardcoded path if available
            if path_from_env != hardcoded_path_from_provider {
                println!(
                    "cargo:warning=Nix path mismatch for {}: Environment provided '{}', hardcoded is '{}'.",
                    env_var, path_from_env, hardcoded_path_from_provider
                );
                println!("cargo:warning=Using path from environment variable: {}", path_from_env);
            } else {
                println!("cargo:warning=Nix path for {} matches hardcoded: {}", env_var, path_from_env);
            }
            path_from_env
        }
        Err(_) => {
            println!(
                "cargo:warning=Environment variable {} not set. Using hardcoded fallback path from provider: {}",
                env_var, hardcoded_path_from_provider
            );
            hardcoded_path_from_provider.to_string()
        }
    }
}

impl NixPaths {
    fn default_nix_paths() -> Self {
        let hardcoded_provider = HardcodedNixPaths;
        NixPaths {
            // --- Basic C/C++ Toolchain Paths ---
            glibc_dev: get_nix_path_from_env(
                "NIX_GLIBC_DEV",
                hardcoded_provider.glibc_dev(),
            ),
            gcc_path: get_nix_path_from_env(
                "NIX_GCC_PATH",
                hardcoded_provider.gcc_path(),
            ),
            gcc_cpp_include: get_nix_path_from_env(
                "NIX_GCC_CPP_INCLUDE",
                hardcoded_provider.gcc_cpp_include(),
            ),

            // --- External Library Include Paths ---
            zlib_include: get_nix_path_from_env(
                "NIX_ZLIB_INCLUDE",
                hardcoded_provider.zlib_include(),
            ),
            bzip2_include: get_nix_path_from_env(
                "NIX_BZIP2_INCLUDE",
                hardcoded_provider.bzip2_include(),
            ),
            lz4_include: get_nix_path_from_env(
                "NIX_LZ4_INCLUDE",
                hardcoded_provider.lz4_include(),
            ),
            lz4_lib: get_nix_path_from_env(
                "NIX_LZ4_LIB",
                hardcoded_provider.lz4_lib(),
            ),
            zstd_include: get_nix_path_from_env(
                "NIX_ZSTD_INCLUDE",
                hardcoded_provider.zstd_include(),
            ),
            zstd_lib: get_nix_path_from_env(
                "NIX_ZSTD_LIB",
                hardcoded_provider.zstd_lib(),
            ),

            // --- LLVM/Clang Paths for Bindgen ---
            llvm_config: get_nix_path_from_env(
                "NIX_LLVM_CONFIG",
                hardcoded_provider.llvm_config(),
            ),
            libclang_path: get_nix_path_from_env(
                "NIX_LIBCLANG_PATH",
                hardcoded_provider.libclang_path(),
            ),
            llvm_config_path: get_nix_path_from_env(
                "NIX_LLVM_CONFIG_PATH",
                hardcoded_provider.llvm_config_path(),
            ),
        }
    }
}


fn generate_snappy_stubs_public_h(out_dir: &Path, snappy_source_dir: &Path) {
    let input_path = snappy_source_dir.join("snappy-stubs-public.h.in");
    let output_path = out_dir.join("snappy-stubs-public.h");

    let mut content = fs::read_to_string(&input_path)
        .expect("Failed to read snappy-stubs-public.h.in");

    // Replace placeholders
    content = content.replace("${HAVE_SYS_UIO_H_01}", "1"); // Assuming Linux, sys/uio.h is present
    content = content.replace("${PROJECT_VERSION_MAJOR}", "1");
    content = content.replace("${PROJECT_VERSION_MINOR}", "2");
    content = content.replace("${PROJECT_VERSION_PATCH}", "2");

    fs::write(&output_path, content)
        .expect("Failed to write snappy-stubs-public.h");
}

// On these platforms jemalloc-sys will use a prefixed jemalloc which cannot be linked together
// with RocksDB.
// See https://github.com/tikv/jemallocator/blob/tikv-jemalloc-sys-0.5.3/jemalloc-sys/src/env.rs#L25
const NO_JEMALLOC_TARGETS: &[&str] = &["android", "dragonfly", "musl", "darwin"];

fn link(name: &str, bundled: bool) {
    use std::env::var;
    let target = var("TARGET").unwrap();
    let target: Vec<_> = target.split('-').collect();
    if target.get(2) == Some(&"windows") {
        println!("cargo:rustc-link-lib=dylib={name}");
        if bundled && target.get(3) == Some(&"gnu") {
            let dir = var("CARGO_MANIFEST_DIR").unwrap();
            println!("cargo:rustc-link-search=native={}/{}", dir, target[0]);
        }
    }
}

fn fail_on_empty_directory(name: &str) {
    if fs::read_dir(name).unwrap().count() == 0 {
        println!("cargo:warning=The `{name}` directory is empty, did you forget to pull the submodules?");
        println!("cargo:warning=Try `git submodule update --init --recursive`");
        panic!();
    }
}

fn rocksdb_include_dir() -> String {
    env::var("ROCKSDB_INCLUDE_DIR").unwrap_or_else(|_| "rocksdb/include".to_string())
}

fn bindgen_rocksdb(nix_paths: &NixPaths) {
    env::set_var("LIBCLANG_PATH", &nix_paths.libclang_path);
    env::set_var("LLVM_CONFIG_PATH", &nix_paths.llvm_config_path);
    env::set_var("LLVM_CONFIG", &nix_paths.llvm_config);
    
    // LIBCLANG_FLAGS is crucial for bindgen to find system headers.
    // Setting it just to --sysroot=<glibc_dev> is consistent with how Nix often
    // configures bindgen via `shellHook` for basic C standard library headers.
    env::set_var("LIBCLANG_FLAGS", &format!("--sysroot={}", nix_paths.glibc_dev));
    
    // BINDGEN_EXTRA_CLANG_ARGS can be used for additional flags, but for now,
    // we rely on LIBCLANG_FLAGS for the essential sysroot setting.
    // Removing previous complex construction to avoid potential conflicts.



    let bindings = bindgen::Builder::default()
        .header(rocksdb_include_dir() + "/rocksdb/c.h")
        .derive_debug(false)
        .blocklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .ctypes_prefix("libc")
        .size_t_is_usize(true)
        .clang_args(&["--verbose"]) // Only --verbose, let LIBCLANG_FLAGS handle includes
        .generate()
        .expect("unable to generate rocksdb bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write rocksdb bindings");
}

fn build_rocksdb(nix_paths: &NixPaths) {
    println!("cargo:warning=Executing build_rocksdb function.");
    let target = env::var("TARGET").unwrap(); // Re-adding this line

        let mut config = cc::Build::new();

        config.cpp(true); // Always compile as C++ for RocksDB

    

        // Explicitly set the compiler

        config.compiler(&format!("{}/bin/g++", nix_paths.gcc_path));

    

        // Explicitly add include paths
        // Use -isystem for Glibc C headers (standard system headers)
        config.flag(&format!("-isystem{}/include", nix_paths.glibc_dev));

        // Use -I for GCC's C++ standard library headers
        config.include(&nix_paths.gcc_cpp_include);

        // Add RocksDB's own include directory
        config.include(rocksdb_include_dir());

        if cfg!(feature = "snappy") {
            config.define("SNAPPY", Some("1"));
            config.include("snappy/");
        }

                if cfg!(feature = "lz4") {

                    config.define("LZ4", Some("1"));

                    config.include(&nix_paths.lz4_include);

                    println!("cargo:rustc-link-search=native={}", nix_paths.lz4_lib); // Add link search path

                    println!("cargo:rustc-link-lib=dylib=lz4"); // Change to dylib linking

                }

        

        

                if cfg!(feature = "zstd") {

        

                    config.define("ZSTD", Some("1"));

        

                    config.include(&nix_paths.zstd_include);

                    println!("cargo:rustc-link-search=native={}", nix_paths.zstd_lib); // Add link search path

                    println!("cargo:rustc-link-lib=dylib=zstd"); // Change to dylib linking

        

                }

    

    if cfg!(feature = "zlib") {
        config.define("ZLIB", Some("1"));
        config.include(&nix_paths.zlib_include);
    }

    if cfg!(feature = "bzip2") {
        config.define("BZIP2", Some("1"));
        config.include(&nix_paths.bzip2_include);
    }

    if cfg!(feature = "rtti") {
        config.define("USE_RTTI", Some("1"));
    }

    // https://github.com/facebook/rocksdb/blob/be7703b27d9b3ac458641aaadf27042d86f6869c/Makefile#L195
    if cfg!(feature = "lto") {
        config.flag("-flto");
        if !config.get_compiler().is_like_clang() {
            panic!(
                "LTO is only supported with clang. Either disable the `lto` feature or set `CC=/usr/bin/clang CXX=/usr/bin/clang++` environment variables."
            );
        }
    }

    config.include(".");
    config.include("rocksdb/");
    config.define("NDEBUG", Some("1"));

    let mut lib_sources = include_str!("rocksdb_lib_sources.txt")
        .trim()
        .split('\n')
        .map(str::trim)
        // We have a pre-generated a version of build_version.cc in the local directory
        .filter(|file| !matches!(*file, "util/build_version.cc"))
        .collect::<Vec<&'static str>>();

    if let (true, Ok(target_feature_value)) = (
        target.contains("x86_64"),
        env::var("CARGO_CFG_TARGET_FEATURE"),
    ) {
        // This is needed to enable hardware CRC32C. Technically, SSE 4.2 is
        // only available since Intel Nehalem (about 2010) and AMD Bulldozer
        // (about 2011).
        let target_features: Vec<_> = target_feature_value.split(',').collect();

        if target_features.contains(&"sse2") {
            config.flag_if_supported("-msse2");
        }
        if target_features.contains(&"sse4.1") {
            config.flag_if_supported("-msse4.1");
        }
        if target_features.contains(&"sse4.2") {
            config.flag_if_supported("-msse4.2");
        }
        // Pass along additional target features as defined in
        // build_tools/build_detect_platform.
        if target_features.contains(&"avx2") {
            config.flag_if_supported("-mavx2");
        }
        if target_features.contains(&"bmi1") {
            config.flag_if_supported("-mbmi");
        }
        if target_features.contains(&"lzcnt") {
            config.flag_if_supported("-mlzcnt");
        }
        if !target.contains("android") && target_features.contains(&"pclmulqdq") {
            config.flag_if_supported("-mpclmul");
        }
    }

    if target.contains("apple-ios") {
        config.define("OS_MACOSX", None);

        config.define("IOS_CROSS_COMPILE", None);
        config.define("PLATFORM", "IOS");
        config.define("NIOSTATS_CONTEXT", None);
        config.define("NPERF_CONTEXT", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);

        env::set_var("IPHONEOS_DEPLOYMENT_TARGET", "12.0");
    } else if target.contains("darwin") {
        config.define("OS_MACOSX", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);
    } else if target.contains("android") {
        config.define("OS_ANDROID", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);

        if &target == "armv7-linux-androideabi" {
            config.define("_FILE_OFFSET_BITS", Some("32"));
        }
    } else if target.contains("aix") {
        config.define("OS_AIX", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);
    } else if target.contains("linux") {
        config.define("OS_LINUX", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);
        config.define("ROCKSDB_SCHED_GETCPU_PRESENT", None);
    } else if target.contains("dragonfly") {
        config.define("OS_DRAGONFLYBSD", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);
    } else if target.contains("freebsd") {
        config.define("OS_FREEBSD", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);
    } else if target.contains("netbsd") {
        config.define("OS_NETBSD", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);
    } else if target.contains("openbsd") {
        config.define("OS_OPENBSD", None);
        config.define("ROCKSDB_PLATFORM_POSIX", None);
        config.define("ROCKSDB_LIB_IO_POSIX", None);
    } else if target.contains("windows") {
        link("rpcrt4", false);
        link("shlwapi", false);
        config.define("DWIN32", None);
        config.define("OS_WIN", None);
        config.define("_MBCS", None);
        config.define("WIN64", None);
        config.define("NOMINMAX", None);
        config.define("ROCKSDB_WINDOWS_UTF8_FILENAMES", None);

        if &target == "x86_64-pc-windows-gnu" {
            // Tell MinGW to create localtime_r wrapper of localtime_s function.
            config.define("_POSIX_C_SOURCE", Some("1"));
            // Tell MinGW to use at least Windows Vista headers instead of the ones of Windows XP.
            // (This is minimum supported version of rocksdb)
            config.define("_WIN32_WINNT", Some("_WIN32_WINNT_VISTA"));
        }

        // Remove POSIX-specific sources
        lib_sources = lib_sources
            .iter()
            .cloned()
            .filter(|file| {
                !matches!(
                    *file,
                    "port/port_posix.cc"
                        | "env/env_posix.cc"
                        | "env/fs_posix.cc"
                        | "env/io_posix.cc"
                )
            })
            .collect::<Vec<&'static str>>();

        // Add Windows-specific sources
        lib_sources.extend([
            "port/win/env_default.cc",
            "port/win/env_win.cc",
            "port/win/io_win.cc",
            "port/win/port_win.cc",
            "port/win/win_logger.cc",
            "port/win/win_thread.cc",
        ]);

        if cfg!(feature = "jemalloc") {
            lib_sources.push("port/win/win_jemalloc.cc");
        }
    }

    config.define("ROCKSDB_SUPPORT_THREAD_LOCAL", None);

    if cfg!(feature = "jemalloc") && NO_JEMALLOC_TARGETS.iter().all(|i| !target.contains(i)) {
        config.define("ROCKSDB_JEMALLOC", Some("1"));
        config.define("JEMALLOC_NO_DEMANGLE", Some("1"));
        if let Some(jemalloc_root) = env::var_os("DEP_JEMALLOC_ROOT") {
            config.include(Path::new(&jemalloc_root).join("include"));
        }
    }

    #[cfg(feature = "io-uring")]
    if target.contains("linux") {
        pkg_config::probe_library("liburing")
            .expect("The io-uring feature was requested but the library is not available");
        config.define("ROCKSDB_IOURING_PRESENT", Some("1"));
    }

    if &target != "armv7-linux-androideabi"
        && env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap() != "64"
    {
        config.define("_FILE_OFFSET_BITS", Some("64"));
        config.define("_LARGEFILE64_SOURCE", Some("1"));
    }

    if target.contains("msvc") {
        if cfg!(feature = "mt_static") {
            config.static_crt(true);
        }
        config.flag("-EHsc");
        config.flag("-std:c++20");
    } else {
        config.flag(cxx_standard());
        // matches the flags in CMakeLists.txt from rocksdb
        config.flag("-Wfatal-errors");
        config.flag("-Wsign-compare");
        config.flag("-Wshadow");
        config.flag("-Wno-unused-parameter");
        config.flag("-Wno-unused-variable");
        config.flag("-Woverloaded-virtual");
        config.flag("-Wnon-virtual-dtor");
        config.flag("-Wno-missing-field-initializers");
        config.flag("-Wno-strict-aliasing");
        config.flag("-Wno-invalid-offsetof");
    }
    if target.contains("riscv64gc") {
        // link libatomic required to build for riscv64gc
        println!("cargo:rustc-link-lib=atomic");
    }
    for file in lib_sources {
        config.file(format!("rocksdb/{file}"));
    }

    config.file("build_version.cc");

    config.cpp(true);
    config.flag_if_supported("-std=c++20");

    if !target.contains("windows") {
        config.flag("-include").flag("cstdint");
        config.flag("-include").flag("cstddef");
    }

    // By default `cc` will link C++ standard library automatically,
    // see https://docs.rs/cc/latest/cc/index.html#c-support.
    // There is no need to manually set `cpp_link_stdlib`.

    config.compile("librocksdb.a");
}

fn build_snappy(nix_paths: &NixPaths) {
    let target = env::var("TARGET").unwrap();
    let endianness = env::var("CARGO_CFG_TARGET_ENDIAN").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let snappy_source_dir = PathBuf::from("snappy"); // Assuming "snappy/" is the base dir for snappy source files

    generate_snappy_stubs_public_h(&out_dir, &snappy_source_dir);

    let mut config = cc::Build::new();
    config.define("NDEBUG", Some("1"));
    config.extra_warnings(false);

    // Explicitly set the compiler
    config.compiler(&format!("{}/bin/g++", nix_paths.gcc_path));

    // Explicitly add include paths
    config.flag(&format!("-isystem{}/include", nix_paths.glibc_dev)); // Glibc C headers
    config.include(&nix_paths.gcc_cpp_include); // GCC C++ headers
    config.include("snappy/"); // Include Snappy's own directory
    config.include(&out_dir); // Include the directory where generated headers are located

    if target.contains("msvc") {
        config.flag("-EHsc");
        if cfg!(feature = "mt_static") {
            config.static_crt(true);
        }
    } else {
        // Snappy requires C++11.
        // See: https://github.com/google/snappy/blob/master/CMakeLists.txt#L32-L38
        config.flag("-std=c++11");
    }

    if endianness == "big" {
        config.define("SNAPPY_IS_BIG_ENDIAN", Some("1"));
    }

    config.file("snappy/snappy.cc");
    config.file("snappy/snappy-sinksource.cc");
    config.file("snappy/snappy-c.cc");
    config.cpp(true);
    config.compile("libsnappy.a");
}

fn try_to_find_and_link_lib(lib_name: &str) -> bool {
    println!("cargo:rerun-if-env-changed={lib_name}_COMPILE");
    if let Ok(v) = env::var(format!("{lib_name}_COMPILE")) {
        if v.to_lowercase() == "true" || v == "1" {
            return false;
        }
    }

    println!("cargo:rerun-if-env-changed={lib_name}_LIB_DIR");
    println!("cargo:rerun-if-env-changed={lib_name}_STATIC");

    if let Ok(lib_dir) = env::var(format!("{lib_name}_LIB_DIR")) {
        println!("cargo:rustc-link-search=native={lib_dir}");
        let mode = match env::var_os(format!("{lib_name}_STATIC")) {
            Some(_) => "static",
            None => "dylib",
        };
        println!("cargo:rustc-link-lib={}={}", mode, lib_name.to_lowercase());
        return true;
    }
    false
}

fn cxx_standard() -> String {
    "-std=c++20".to_owned()
}

fn update_submodules() {
    let program = "git";
    let dir = "../";
    let args = ["submodule", "update", "--init"];
    println!(
        "Running command: \"{} {}\" in dir: {}",
        program,
        args.join(" "),
        dir
    );
    let ret = Command::new(program).current_dir(dir).args(args).status();

    match ret.map(|status| (status.success(), status.code())) {
        Ok((true, _)) => (),
        Ok((false, Some(c))) => panic!("Command failed with error code {c}"),
        Ok((false, None)) => panic!("Command got killed"),
        Err(e) => panic!("Command failed with error: {e}"),
    }
}

fn cpp_link_stdlib(target: &str) {
    // according to https://github.com/alexcrichton/cc-rs/blob/master/src/lib.rs#L2189
    if let Ok(stdlib) = env::var("CXXSTDLIB") {
        println!("cargo:rustc-link-lib=dylib={stdlib}");
    } else if target.contains("apple") || target.contains("freebsd") || target.contains("openbsd") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    } else if target.contains("aix") {
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=dylib=c++abi");
    }
}

fn main() {
    // Aggressively unset environment variables that could interfere with the build process
    // This ensures that `build.rs` is completely self-contained and deterministic,
    // not relying on or being influenced by `flake.nix`'s `shellHook` or other external settings.
    // env::remove_var("CC"); // Removed to allow propagation
    // env::remove_var("CXX"); // Removed to allow propagation
    // env::remove_var("CFLAGS"); // Removed to allow propagation
    // env::remove_var("CXXFLAGS"); // Removed to allow propagation

    // env::remove_var("LIBRARY_PATH"); // Removed to allow propagation
    // env::remove_var("LD_LIBRARY_PATH"); // Removed to allow propagation
    // env::remove_var("PROTOC"); // Removed to allow propagation
    // env::remove_var("PROTOC_INCLUDE"); // Removed to allow propagation
    // env::remove_var("BINDGEN_EXTRA_CLANG_ARGS"); // Removed to allow propagation
    // env::remove_var("LLVM_CONFIG"); // Removed to allow propagation
    // env::remove_var("LLVM_CONFIG_PATH"); // Removed to allow propagation
    // env::remove_var("LIBCLANG_PATH"); // Removed to allow propagation
    // env::remove_var("LIBCLANG_FLAGS"); // Removed to allow propagation

    // env::remove_var("NIX_GLIBC_DEV"); // Removed to allow propagation
    // env::remove_var("NIX_GCC_PATH"); // Removed to allow propagation
    // env::remove_var("NIX_GCC_REAL_PATH"); // Removed to allow propagation

    // Always use hardcoded Nix paths for standalone compilation, and defensively unset
    // environment variables that might interfere.
    let nix_paths = NixPaths::default_nix_paths();

    // --- Defensive Unsetting of Environment Variables ---
    // These environment variables are commonly set by Nix shellHooks or other build systems.
    // Unsetting them ensures that our hardcoded paths (or future dynamically extracted paths)
    // are used consistently, preventing unexpected behavior or conflicts.
    // env::remove_var("CXX"); // Now set explicitly later
    env::remove_var("CXXFLAGS"); // Keep removed, we will set it after
    env::remove_var("CPATH");
    env::remove_var("C_INCLUDE_PATH");
    env::remove_var("CPLUS_INCLUDE_PATH");
    // env::remove_var("CC"); // Now set explicitly later
    env::remove_var("CFLAGS");
    env::remove_var("LIBRARY_PATH");
    env::remove_var("LD_LIBRARY_PATH");
    env::remove_var("PROTOC");
    env::remove_var("PROTOC_INCLUDE");
    env::remove_var("BINDGEN_EXTRA_CLANG_ARGS");
    env::remove_var("LLVM_CONFIG");
    env::remove_var("LLVM_CONFIG_PATH");
    env::remove_var("LIBCLANG_PATH");
    env::remove_var("LIBCLANG_FLAGS");
    env::remove_var("NIX_GLIBC_DEV");
    env::remove_var("NIX_GCC_PATH");
    env::remove_var("NIX_GCC_REAL_PATH");
    // Ensure that pkg-config doesn't interfere with our explicit path settings
    env::remove_var("PKG_CONFIG_PATH"); 


    // --- Setting Environment Variables for Build Tools ---
    // These variables are set explicitly here to ensure that `cc-rs` and `bindgen`
    // use the correct Nix-derived toolchain and include paths for standalone builds.
    // This replicates the environment typically provided by a Nix `devShell`.
    env::set_var("CC", &format!("{}/bin/gcc", nix_paths.gcc_path));
    env::set_var("CXX", &format!("{}/bin/g++", nix_paths.gcc_path));
    env::set_var("CPATH", ""); // Set to empty string to avoid conflicts
    env::set_var("CXXFLAGS", ""); // Set to empty string to avoid conflicts




    if !Path::new("rocksdb/AUTHORS").exists() {
        update_submodules();
    }
    bindgen_rocksdb(&nix_paths); // Pass nix_paths to bindgen_rocksdb
    env::remove_var("LIBCLANG_PATH");
    env::remove_var("LLVM_CONFIG_PATH");
    env::remove_var("LLVM_CONFIG");
    env::remove_var("LIBCLANG_FLAGS");
    let target = env::var("TARGET").unwrap();

    if !try_to_find_and_link_lib("ROCKSDB") {
        // rocksdb only works with the prebuilt rocksdb system lib on freebsd.
        // we don't need to rebuild rocksdb
        if target.contains("freebsd") {
            println!("cargo:rustc-link-search=native=/usr/local/lib");
            let mode = match env::var_os("ROCKSDB_STATIC") {
                Some(_) => "static",
                None => "dylib",
            };
            println!("cargo:rustc-link-lib={mode}=rocksdb");

            return;
        }

        println!("cargo:rerun-if-changed=rocksdb/");
        fail_on_empty_directory("rocksdb");
        build_rocksdb(&nix_paths); // Pass nix_paths to build_rocksdb
    } else {
        cpp_link_stdlib(&target);
    }
    if cfg!(feature = "snappy") && !try_to_find_and_link_lib("SNAPPY") {
        println!("cargo:rerun-if-changed=snappy/");
        fail_on_empty_directory("snappy");
        build_snappy(&nix_paths); // Pass nix_paths to build_snappy
    }

    // Allow dependent crates to locate the sources and output directory of
    // this crate. Notably, this allows a dependent crate to locate the RocksDB
    // sources and built archive artifacts provided by this crate.
    println!(
        "cargo:cargo_manifest_dir={}",
        env::var("CARGO_MANIFEST_DIR").unwrap()
    );
    println!("cargo:out_dir={}", env::var("OUT_DIR").unwrap());
}
