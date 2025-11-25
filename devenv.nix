{ pkgs, lib, ... }:
{
  env = {
    NIX_STORE = "/nix/store";
    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    LD_LIBRARY_PATH = lib.makeLibraryPath [
      pkgs.libclang.lib
      pkgs.libxkbcommon
      pkgs.wayland
    ];
  };

  packages = with pkgs; [
    rustup
    cargo-bloat
    cargo-deny
    cargo-zigbuild
    cargo-watch
    cargo-nextest
    zig
    docker
    sdl2-compat
    libclang
    inetutils
  ];

  languages.rust = {
    enable = true;
    channel = "nightly";
    targets = [
      "armv7-unknown-linux-gnueabihf"
    ];
  };
}
