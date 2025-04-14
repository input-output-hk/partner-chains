{
  self,
  inputs,
  system,
  ...
}: {
  perSystem = {
    inputs',
    self',
    lib,
    pkgs,
    system,
    ...
  }: let
    flake-compat = import inputs.flake-compat;
    cardanoPackages = (flake-compat { src = inputs.cardano-node; }).defaultNix.packages.${system};
    dbSyncPackages = (flake-compat { src = inputs.cardano-dbsync; }).defaultNix.packages.${system};
    fenixPkgs = inputs'.fenix.packages;
    # rustToolchainFile = with fenixPkgs;
    #   fromToolchainFile {
    #       file = ../../../rust-toolchain.toml;
    #       sha256 = "VZZnlyP69+Y3crrLHQyJirqlHrTtGTsyiSnZB8jEvVo=";
    #     };
    rustToolchain = fenixPkgs.combine [
      #rustToolchainFile
      fenixPkgs.latest.toolchain
      fenixPkgs.latest.rust-src
      fenixPkgs.targets.wasm32-unknown-unknown.latest.rust-std
    ];
    customRustPlatform = pkgs.makeRustPlatform {
      cargo = rustToolchain;
      rustc = rustToolchain;
    };

  in {
    packages = {
      inherit (cardanoPackages) cardano-node cardano-cli cardano-testnet;
      inherit (dbSyncPackages) "cardano-db-sync:exe:cardano-db-sync";
      ogmios = pkgs.callPackage ./ogmios.nix { };
      process-compose = pkgs.process-compose.overrideAttrs (oldAttrs: {
        patches = [ ./pc.patch ];
      });
      partnerchains-stack = pkgs.callPackage ./partnerchains-stack { inherit (self'.packages) partnerchains-stack-unwrapped; };
      partnerchains-stack-unwrapped = pkgs.callPackage ./partnerchains-stack-unwrapped { };
      partner-chains = 
      let 
        # Create a patched source
        patchedSrc = pkgs.stdenv.mkDerivation {
          name = "partner-chains-patched-src";
          src = ../../../.;
          
          # Apply patch with fuzz factor to handle offsets and force option to continue despite failed hunks
          patchFlags = ["-p1" "--fuzz=3" "--force"];
          patches = [./rust-src-std.patch];
          
          # Add a build phase to patch wasm-opt-sys and unwinding crates
          # buildPhase = ''
          #   # Patch wasm-opt-sys to force C++17 support
          #   WASM_OPT_BUILD_RS=$(find . -type f -path "*/wasm-opt-sys-0.116.0/build.rs")
          #   if [ -n "$WASM_OPT_BUILD_RS" ]; then
          #     echo "Patching $WASM_OPT_BUILD_RS for C++17 support"
          #     sed -i 's/flag_if_supported("-std=c++17")/flag("-std=c++17")/' "$WASM_OPT_BUILD_RS"
          #   fi
          # '';
          
          installPhase = ''
            cp -r . $out
          '';
        };
      in customRustPlatform.buildRustPackage rec {
        pname = "partner-chains";
        version = "1.6";
        src = patchedSrc;
        
        cargoLock = {
          lockFile = "${patchedSrc}/Cargo.lock";
          outputHashes = {
            "binary-merkle-tree-16.0.0" = "sha256-E7Mq/0EwqVSnJ6nX7TZppLjwje7vYuxjjCkt5kWQjfQ=";
            "pallas-addresses-0.31.0" = "sha256-gecokNSH018NSFb8Cm+Gvql8wkip0t/IkdgY3ZRILbE=";
            "raw-scripts-7.0.2" = "sha256-rUms2AZGOEjh1/zvfVoqv/B1XFUiglDf9UAMaqFIQZU=";
          };
        };
        
        doCheck = false;
        patches = [];
        
        # Add files to support cargo patching
        postUnpack = ''
          # Create .cargo directory
          mkdir -p "$sourceRoot/.cargo"
          
          # Create config that patches the unwinding crate
          cat > "$sourceRoot/.cargo/config.toml" << 'EOF'
          [patch.crates-io]
          unwinding = { path = "./.cargo/unwinding-patched" }
          EOF
          
          # Create directory structure for patched unwinding
          mkdir -p "$sourceRoot/.cargo/unwinding-patched/src/unwinder/find_fde"
          
          # Create Cargo.toml with ALL required features
          cat > "$sourceRoot/.cargo/unwinding-patched/Cargo.toml" << 'EOF'
          [package]
          name = "unwinding"
          version = "0.2.5"
          edition = "2021"
          
          [dependencies]
          libc = "0.2"
          cfg-if = "1.0"
          
          [features]
          default = []
          fde-registry = []
          unwinding_apple = []
          static = []
          fde-custom = []
          rustc-dep-of-std = []
          unwinder = []  # Added this feature required by the standard library
          EOF
          
          # Create lib.rs with proper no_std attributes
          cat > "$sourceRoot/.cargo/unwinding-patched/src/lib.rs" << 'EOF'
          #![no_std]
          #![allow(unused_imports)]
          #![feature(strict_provenance)]
          
          extern crate alloc;
          
          pub mod abi;
          pub mod unwinder;
          EOF
          
          # Create unwinder/mod.rs with all required modules
          mkdir -p "$sourceRoot/.cargo/unwinding-patched/src/unwinder"
          cat > "$sourceRoot/.cargo/unwinding-patched/src/unwinder/mod.rs" << 'EOF'
          pub mod find_fde;
          
          // Empty implementations of other required modules
          pub mod registers {
              // Empty implementation that compiles on all platforms
          }
          
          pub mod resume {
              // Empty implementation that compiles on all platforms  
          }
          
          pub mod personality {
              // Empty implementation that compiles on all platforms
          }
          EOF
          
          # Create unwinder/find_fde/mod.rs with cross-platform support
          mkdir -p "$sourceRoot/.cargo/unwinding-patched/src/unwinder/find_fde"
          cat > "$sourceRoot/.cargo/unwinding-patched/src/unwinder/find_fde/mod.rs" << 'EOF'
          pub struct EhRef {
              pub eh_frame_ptr: *const u8,
              pub eh_frame_size: usize,
          }
          
          #[cfg(not(target_os = "macos"))]
          mod phdr;
          
          #[cfg(not(target_os = "macos"))]
          pub use phdr::find_eh_frame;
          
          #[cfg(target_os = "macos")]
          pub fn find_eh_frame() -> Option<EhRef> {
              None
          }
          EOF
          
          # Create cross-platform-compatible phdr.rs that only compiles on Linux
          cat > "$sourceRoot/.cargo/unwinding-patched/src/unwinder/find_fde/phdr.rs" << 'EOF'
          #![cfg(not(target_os = "macos"))]
          
          use alloc::vec::Vec;
          use core::ffi::c_void;
          use core::mem;
          use core::ptr;
          use core::slice;
          use crate::unwinder::find_fde::EhRef;
          
          // Define an empty find_eh_frame function for non-macOS
          pub fn find_eh_frame() -> Option<EhRef> {
              None
          }
          EOF
          
          # Create abi.rs module with minimal required types
          cat > "$sourceRoot/.cargo/unwinding-patched/src/abi.rs" << 'EOF'
          // Common types for unwinding ABI
          
          pub enum Reason {
              Panic,
              ForeignException,
              Unknown,
          }
          
          pub type Callback<R> = fn(&R) -> !;
          EOF
          
          echo "Created patched unwinding crate at $sourceRoot/.cargo/unwinding-patched"
          find "$sourceRoot/.cargo/unwinding-patched" -type f | sort
        '';
        
        # Patch unwinding crate in cargo-vendor-dir as a backup if the cargo patch doesn't work
        preBuild = ''
          # Patch vendored unwinding find_fde/mod.rs
          FIND_FDE_MOD=$(find . -type f -path "*/unwinding-0.2.5/src/unwinder/find_fde/mod.rs" | head -n 1)
          if [ -n "$FIND_FDE_MOD" ]; then
            echo "Found unwinding find_fde/mod.rs at: $FIND_FDE_MOD"
            # Replace the entire file
            cat > "$FIND_FDE_MOD" << 'EOF'
          pub struct EhRef {
              pub eh_frame_ptr: *const u8,
              pub eh_frame_size: usize,
          }
          
          #[cfg(not(target_os = "macos"))]
          mod phdr;
          
          #[cfg(not(target_os = "macos"))]
          pub use phdr::find_eh_frame;
          
          #[cfg(target_os = "macos")]
          pub fn find_eh_frame() -> Option<EhRef> {
              None
          }
          EOF
          fi
          
          # Also try to patch the phdr.rs file directly
          UNWINDING_PHDR=$(find . -type f -path "*/unwinding-0.2.5/src/unwinder/find_fde/phdr.rs" | head -n 1)
          if [ -n "$UNWINDING_PHDR" ]; then
            echo "Found unwinding phdr.rs at: $UNWINDING_PHDR"
            # Add cfg attribute at the top of the file
            sed -i '1i #![cfg(not(target_os = "macos"))]' "$UNWINDING_PHDR"
          fi
        '';
        
        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.protobuf
          #cWrapper  # Use our custom C wrapper for C code
          #cxxWrapper  # Use our custom C++ wrapper for C++ code
          pkgs.llvmPackages.lld
          customRustPlatform.bindgenHook
        ];
        buildInputs = [
          pkgs.rocksdb
          pkgs.openssl
          pkgs.libclang.lib
        ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.rust-jemalloc-sys-unprefixed ]
        ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
        ];
        
        postFixup = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
          patchelf --set-rpath "${pkgs.rocksdb}/lib:${pkgs.stdenv.cc.cc.lib}/lib" $out/bin/partner-chains-demo-node
        '';
        
        # Force skip support check in CC crate
        CRATE_CC_NO_DEFAULTS = "1";
        
        # Platform-specific features
        RUSTFLAGS = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isDarwin
          "--cfg unwinding_backport --cfg unwinding_apple";
        
        # Override cargo features for unwinding crate (if needed)
        CARGO_FEATURE_UNWINDING_APPLE = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isDarwin "1";
        
        # Existing environment variables
        CC_ENABLE_DEBUG_OUTPUT = "1";
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.llvmPackages.lld}/bin/lld";
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
          rustToolchain
          pkgs.stdenv.cc.cc
          pkgs.libz
          pkgs.clang
        ];
        
        # Platform-specific flags
        CFLAGS =
          if pkgs.lib.hasSuffix "linux" system then
            "-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE"
          else
            "";
            
        PROTOC = "${pkgs.protobuf}/bin/protoc";
        ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
        OPENSSL_NO_VENDOR = 1;
        OPENSSL_DIR = "${pkgs.openssl.dev}";
        OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.cc}/include -std=c++17";

      };
    };
  };
}
    


    