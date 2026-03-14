{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          linuxRuntimeLibraries =
            with pkgs;
            [
              alsa-lib
              expat
              fontconfig
              glib
              libdrm
              libffi
              libglvnd
              libgit2
              libxkbcommon
              llvmPackages.libllvm
              mesa
              stdenv.cc.cc.lib
              vulkan-loader
              wayland
              libx11
              libxcursor
              libxdamage
              libxext
              libxfixes
              libxi
              libxrandr
              libxrender
              libxcb
              libxshmfence
              zlib
              zstd
            ];
        in
        {
          default = pkgs.mkShell {
            packages =
              with pkgs;
              [
                cargo
                rustc
                clippy
                rustfmt
                just
                openssl
                pkgconf
              ]
              ++ lib.optionals stdenv.isLinux [
                gcc
                gnumake
                clang
                cmake
                alsa-lib
                expat
                fontconfig
                libgit2
                glib
                vulkan-loader
                wayland
                libx11
                libxcb
                libxkbcommon
                zstd
                patchelf
              ];

            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            LD_LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isLinux (
              pkgs.lib.makeLibraryPath linuxRuntimeLibraries
            );
            LIBGL_DRIVERS_PATH = pkgs.lib.optionalString pkgs.stdenv.isLinux (
              "${pkgs.mesa}/lib/dri"
            );
          };
        }
      );
    };
}
