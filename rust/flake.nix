{
  description = "A dev shell for Rust development";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        custom-rust-bin = pkgs.rust-bin.stable."1.95.0".default.override {
          extensions = [
            # Required by rfd
            "rust-src"
            "rust-analyzer"
          ];
        };
        mingPkgs = pkgs.pkgsCross.mingwW64;
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            custom-rust-bin
            pkg-config
            eza
            fd
            openssl
            # For Rusty File Dialogs
            wayland
            xdg-desktop-portal-gtk
            # For serialport
            udev
          ];

          shellHook = ''
            alias ls=eza
            alias find=fd
            # Required for DBus to work in COSMIC
            # https://github.com/PolyMeilex/rfd/issues/305#issuecomment-3766284352
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.dbus.lib.outPath}/lib"
          '';
        };
        packages.host_tui_windows = mingPkgs.rustPlatform.buildRustPackage {
          pname = "host_tui_windows";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "--bin"
            "host_tui"
          ];
        };
      }
    );
}
