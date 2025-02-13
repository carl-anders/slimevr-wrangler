{
  description = "Joycon bridge for SlimeVR ecosystem";

  inputs.nixpkgs.url = "nixpkgs/nixos-22.11";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem
    (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        nativeBuildInputs = with pkgs; [
          curl
          gcc
          openssl
          pkgconfig
          which
          zlib

          rust-bin.stable.latest.default
          freetype
          expat
        ];
        buildInputs = with pkgs; [
          appimagekit
          atk
          cairo
          dbus
          dbus.lib
          glib.out
          openssl.out
          pkg-config
          treefmt
          zlib
          systemd
          llvm
          llvmPackages.libclang
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libGL
          freetype
          freetype.dev
          vulkan-headers
          vulkan-loader
          vulkan-tools
          expat

          # Some nice things to have
          exa
          fd
        ];
      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs =
            nativeBuildInputs
            ++ [
            ];

          buildInputs =
            buildInputs
            ++ [
            ];

          shellHook = ''
            # bindgen requires CLang stuff and this was it
            export BINDGEN_EXTRA_CLANG_ARGS="$(< ${pkgs.stdenv.cc}/nix-support/libc-crt1-cflags) \
            $(< ${pkgs.stdenv.cc}/nix-support/libc-cflags) \
            $(< ${pkgs.stdenv.cc}/nix-support/cc-cflags) \
            $(< ${pkgs.stdenv.cc}/nix-support/libcxx-cxxflags) \
            ${
              pkgs.lib.optionalString pkgs.stdenv.cc.isClang
              "-idirafter ${pkgs.stdenv.cc.cc}/lib/clang/${
                pkgs.lib.getVersion pkgs.stdenv.cc.cc
              }/include"
            } \
            ${
              pkgs.lib.optionalString pkgs.stdenv.cc.isGNU
              "-isystem ${pkgs.stdenv.cc.cc}/include/c++/${
                pkgs.lib.getVersion pkgs.stdenv.cc.cc
              } -isystem ${pkgs.stdenv.cc.cc}/include/c++/${
                pkgs.lib.getVersion pkgs.stdenv.cc.cc
              }/${pkgs.stdenv.hostPlatform.config} -idirafter ${pkgs.stdenv.cc.cc}/lib/gcc/${pkgs.stdenv.hostPlatform.config}/${
                pkgs.lib.getVersion pkgs.stdenv.cc.cc
              }/include"
            } \
            "
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
            export LD_LIBRARY_PATH="${builtins.foldl'
              (a: b: "${a}:${b}/lib") "${pkgs.vulkan-loader}/lib"
              buildInputs}";
            alias ls=exa
            alias find=fd
          '';
        };
      }
    );
}
