{
  description = "Nixification of the rust-rocksdb/librocksdb-sys crate";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable"; # Directly use nixpkgs-unstable
    flake-utils.url = "github:meta-introspector/flake-utils?ref=feature/CRQ-016-nixify";
    # Temporarily remove rust-overlay to debug
    # rust-overlay = {
    #   url = "github:meta-introspector/rust-overlay?ref=feature/CRQ-016-nixify";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
  };

  outputs = { self, nixpkgs, flake-utils }: # Removed rust-overlay from arguments
    flake-utils.lib.eachDefaultSystem (system:
      let
        # overlays = [ rust-overlay.overlays.default ]; # Removed rust-overlay
        pkgs = import nixpkgs {
          inherit system; # Removed overlays
          config = {
            permittedInsecurePackages = [ "openssl-1.1.1w" ];
          };
        };

        # Use default rustc from nixpkgs for now
        myRustc = pkgs.rustc;

      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs;
            [ # Rust toolchain
              myRustc
              cargo
              rustfmt
              clippy

              # Essential build tools for C/C++ and FFI
              clang # For C++ compilation and bindgen
              llvmPackages.libclang # For bindgen
              llvmPackages.llvm # Provides llvm-config
              gcc # C compiler
              glibc.dev # Provides system headers

              # Libraries corresponding to Cargo features
              snappy # For "snappy" feature
              lz4 # For "lz4" feature
              zstd # For "zstd" feature
              zlib # For "zlib" feature
              bzip2 # For "bzip2" feature
              liburing # For "io-uring" feature (pkg-config will find it)
              pkg-config # Needed for pkg-config in build.rs
              openssl_1_1.dev # Included in permittedInsecurePackages, but needs to be in packages for its headers to be found by bindgen.
            ];

          shellHook = ''
            # Environment variables needed by build.rs
            # These should guide bindgen and cc-rs to the correct Nix paths
            export LIBCLANG_PATH="/nix/store/10mkp77lmqz8x2awd8hzv6pf7f7rkf6d-clang-19.1.7-lib/lib"; # Explicit path
            export LLVM_CONFIG_PATH="${pkgs.llvmPackages.llvm}/lib";
            export LLVM_CONFIG="${pkgs.llvmPackages.llvm.dev}/bin/llvm-config";
            
            # CFLAGS and CXXFLAGS for the C/C++ compiler used by cc-rs
            # These are inherited from the devShell in oldflake.nix
            # and should be sufficient for general compilation.
            export CFLAGS="-O2 -g";
            export CXXFLAGS="-O2 -g -isystem ${pkgs.glibc.dev}/include -isystem ${pkgs.gcc}/include/c++/${pkgs.gcc.version}";

            # Flags for bindgen to find system headers.
            # This incorporates the logic from oldflake.nix shellHook.
            export BINDGEN_EXTRA_CLANG_ARGS="$(
              cat ${pkgs.stdenv.cc}/nix-support/libc-crt1-cflags \
                   ${pkgs.stdenv.cc}/nix-support/libc-cflags \
                   ${pkgs.stdenv.cc}/nix-support/cc-cflags) \
            ${pkgs.lib.optionalString pkgs.stdenv.cc.isClang "-idirafter ${pkgs.stdenv.cc.cc.lib}/lib/clang/${pkgs.lib.getVersion pkgs.stdenv.cc.cc}/include"}"

            # Ensure cargo is available in PATH for cargo build inside nix develop
            export PATH="${myRustc}/bin:${pkgs.cargo}/bin:$PATH";
            
            # Additional environment variables from oldflake.nix
            export PKG_CONFIG_PATH="${pkgs.openssl_1_1.dev}/lib/pkgconfig''${PKG_CONFIG_PATH:+:}$PKG_CONFIG_PATH";
            export REAL_LIBRARY_PATH_VAR="LD_LIBRARY_PATH";
            export REAL_LIBRARY_PATH="$LD_LIBRARY_PATH";
            export CPATH="${pkgs.glibc.dev}/include:${pkgs.gcc}/include"; # Simplified CPATH for debugging
            export RUSTC_BOOTSTRAP=1;
            export LIBCLANG_FLAGS="--sysroot=${pkgs.glibc.dev}";
            # For bindgen to find stdbool.h
            export CFG_RELEASE="1.70.0"; # Added to resolve rustc_hir error

            echo "Nix development shell for librocksdb-sys is ready. Run 'cargo build' to compile.";
          '';
        };
      }
    );
}
