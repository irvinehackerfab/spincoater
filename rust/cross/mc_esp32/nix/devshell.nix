{
  config,
  lib,
  pkgs,
  ...
}:
{
  devshells.default =
    let
      inherit (config) packages;
      chosen.rustc = if pkgs.stdenv.isLinux then packages.bwrap-rustc else packages.unsafe-bin-esp-rust;
      chosen.rustdoc =
        if pkgs.stdenv.isLinux then packages.bwrap-rustdoc else packages.unsafe-bin-esp-rust;
    in
    { config, ... }:
    {
      name = "mc_esp32";

      commands = [
        { package = chosen.rustc; }
        { package = packages.cargo-any-rust; }
        { package = pkgs.rustfmt; }
        { package = pkgs.rust-analyzer; }
        { package = pkgs.clippy; }
        { package = pkgs.espflash; }
        { package = pkgs.picocom; }
      ];

      env = [
        {
          name = "RUSTC";
          value = lib.getExe chosen.rustc;
        }
        {
          name = "CARGO_HOME";
          # We need it to be under $PRJ_ROOT, so that the sandboxed `rustc` has
          # access to it.
          eval = ''"$PRJ_ROOT"/target/cargo-home'';
        }
        {
          name = "RUST_SRC_PATH";
          value = "${packages.esp-rust-src}/lib/rustlib/src/rust/library";
        }
        {
          name = "ESPFLASH_SKIP_UPDATE_CHECK";
          value = "true";
        }
      ];

      devshell = {
        packages = [
          pkgs.unixtools.xxd
          chosen.rustdoc
        ];

        motd = ''

          {202}🔨 Welcome to ${config.name}{reset}

          Untrusted binary blobs (pre-built Rust and GCC compilers) are run in a strict Bubblewrap
          ({bold}bwrap{reset}) sandbox with access only to {bold}$PRJ_ROOT{reset}.

          The other tools (Cargo, espflash, etc.) are source-based and come from regular Nixpkgs.
          $(menu)

          You can now run:
            • {bold}cargo build
            • {bold}cargo doc --open{reset}
            • {bold}espflash save-image --chip esp32 target/xtensa-esp32-none-elf/debug/spincoater out.bin{reset}

          To flash, and monitor output:
            • {bold}cargo run --release{reset} (alias of ^)
            • {bold}picocom --baud=115200 --imap lfcrlf /dev/ttyUSB0{reset}
        '';

        startup.verify-bwrap.text = lib.getExe packages.verify-bwrap;
      };
    };
}
