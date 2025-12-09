use std::env;

#[derive(Debug, Clone)]
struct NixPaths {
    glibc_dev: String,
    gcc_path: String,
    gcc_cpp_include: String,
    zlib_include: String,
    bzip2_include: String,
    lz4_include: String,
    zstd_include: String,
}

impl NixPaths {
    fn default_nix_paths() -> Self {
        println!("cargo:warning=Using hardcoded default Nix paths.");
        NixPaths {
            glibc_dev: "/nix/store/gi4cz4ir3zlwhf1azqfgxqdnczfrwsr7-glibc-2.40-66-dev".to_string(),
            gcc_path: "/nix/store/82kmz7r96navanrc2fgckh2bamiqrgsw-gcc-14.3.0".to_string(),
            gcc_cpp_include: "/nix/store/82kmz7r96navanrc2fgckh2bamiqrgsw-gcc-14.3.0/include/c++/14.3.0".to_string(),
            zlib_include: "/nix/store/hqvsiah013yzb17b13fn18fpqk7m13cg-zlib-1.3.1-dev/include".to_string(),
            bzip2_include: "/nix/store/q1a3bjhg3b4plgb7fk7zis1gi09rbi1d-bzip2-1.0.8-dev/include".to_string(),
            lz4_include: "/nix/store/somehash-lz4-1.9.4-dev/include".to_string(), // Placeholder
            zstd_include: "/nix/store/somehash-zstd-1.5.5-dev/include".to_string(), // Placeholder
        }
    }
}

fn main() {
    // Unset potentially interfering environment variables
    env::remove_var("CXX");
    env::remove_var("CXXFLAGS");
    env::remove_var("CPATH");
    env::remove_var("C_INCLUDE_PATH");
    env::remove_var("CPLUS_INCLUDE_PATH");

    let nix_paths = NixPaths::default_nix_paths();
    
    let mut build = cc::Build::new();
    build.cpp(true); // Compile as C++
    build.file("src/test.cpp"); // Our simple C++ file
    
    // Explicitly set the compiler and flags
    build.compiler(&format!("{}/bin/g++", nix_paths.gcc_path));

    // Explicitly add include paths
    build.flag(&format!("-isystem{}/include", nix_paths.glibc_dev)); // Glibc C headers
    build.include(&nix_paths.gcc_cpp_include); // GCC C++ headers

    // Add conditional defines and includes for external libraries
    if cfg!(feature = "zlib") {
        build.define("ZLIB", Some("1"));
        build.include(&nix_paths.zlib_include);
    }
    if cfg!(feature = "bzip2") {
        build.define("BZIP2", Some("1"));
        build.include(&nix_paths.bzip2_include);
    }
    if cfg!(feature = "lz4") {
        build.define("LZ4", Some("1"));
        build.include(&nix_paths.lz4_include);
    }
    if cfg!(feature = "zstd") {
        build.define("ZSTD", Some("1"));
        build.include(&nix_paths.zstd_include);
    }

    build.compile("test_cc_rs_cxx"); // Name of the static library to be created
}