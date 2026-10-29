{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  # Expose cross tools without importing their setup hook into native C builds.
  mingwTools = pkgs.buildEnv {
    name = "valheim-mingw-tools";
    paths = [ pkgs.pkgsCross.mingwW64.stdenv.cc pkgs.pkgsCross.mingwW64.stdenv.cc.bintools ];
    pathsToLink = [ "/bin" ];
  };
  libraries = with pkgs; [
    libGL
    libxkbcommon
    wayland
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    xorg.libXrender
    xorg.libXext
    xorg.libXfixes
    xorg.libXinerama
    xorg.libxcb
    vulkan-loader
    fontconfig
    zlib
  ];
in
{
  # Language runtimes
  languages.rust = {
    enable = true;
    channel = "stable";
    version = "1.98.1";
    targets = [ "x86_64-pc-windows-gnu" ];
  };
  languages.javascript.enable = true;
  languages.python.enable = true;

  # Project dependencies and tools
  packages = with pkgs; [
    bash
    coreutils
    findutils
    gawk
    gnugrep
    gnused
    git
    gh
    ripgrep
    binutils
    patchelf
    glibc.bin
    shellcheck
    actionlint
    cargo-audit
    util-linux
    xorg.xorgserver
    xorg.xwininfo
    xkeyboard_config
    pkg-config
    zip
    unzip
    jq
    curl
    file
    perl
    gnumake
    zlib
    squashfsTools
    appimage-run
    mingwTools
  ] ++ libraries;

  # Dynamic linker path for GUI crates (winit, eframe, glow)
  env.LD_LIBRARY_PATH = lib.makeLibraryPath libraries;
  env.APPIMAGE_XKB_ROOT = "${pkgs.xkeyboard_config}/share/X11/xkb";
  env.APPIMAGE_XLOCALE_ROOT = "${pkgs.xorg.libX11}/share/X11/locale";
  env.MINGW_PTHREAD_LIB = "${pkgs.pkgsCross.mingwW64.windows.pthreads}/lib/libpthread.a";
  # Keep native C builds native; the Windows script sets target-specific tools.
  env.CC = "${pkgs.stdenv.cc}/bin/cc";
  env.AR = "${pkgs.stdenv.cc.bintools}/bin/ar";
  env.RANLIB = "${pkgs.stdenv.cc.bintools}/bin/ranlib";

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
