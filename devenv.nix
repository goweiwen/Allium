{ pkgs, lib, ... }:
{
  env = {
    NIX_STORE = "/nix/store";
    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.libclang.lib ];
  };

  packages = with pkgs; [
    rustup
    cargo-bloat
    cargo-zigbuild
    cargo-watch
    cargo-nextest
    zig
    docker
    sdl2-compat # Simulator currently crashes immediately: https://github.com/libsdl-org/sdl2-compat/issues/508
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
