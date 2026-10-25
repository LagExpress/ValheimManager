{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  libraries = with pkgs; [
    libGL
    libxkbcommon
    wayland
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    vulkan-loader
    fontconfig
  ];
in
{
  # Language runtimes
  languages.rust.enable = true;
  languages.javascript.enable = true;

  # Project dependencies and tools
  packages = with pkgs; [
    pkg-config
    zip
    unzip
    jq
    curl
    file
    squashfsTools
    appimage-run
  ] ++ libraries;

  # Dynamic linker path for GUI crates (winit, eframe, glow)
  env.LD_LIBRARY_PATH = lib.makeLibraryPath libraries;

  # Helper scripts
  scripts.pack.exec = ''
    node valheim-pack.js "$@"
  '';
  scripts.gui.exec = ''
    cargo run --release "$@"
  '';
  scripts.appimage.exec = ''
    ./scripts/build-appimage.sh "$@"
  '';
}
