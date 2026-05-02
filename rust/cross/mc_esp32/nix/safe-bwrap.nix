{
  config,
  lib,
  pkgs,
  ...
}: let
  bubble-wrap-rust = exe: let
    unsafe = config.packages.unsafe-bin-esp-rust;
    closure = pkgs.writeClosure (pkgs.writeText "all-deps" (toString [unsafe cargo-any-rust]));
    strictStore = true;
  in
    pkgs.writeShellApplication {
      name = exe;
      meta.description = "Pre-built Rust compiler fork for ESP32 (xtensa, riscv32) in a Bubblewrap sandbox (${exe})";
      text = ''
        mkdir -p "$PRJ_ROOT/target/tmp"
        bwrap_opts=(
          --unshare-all
          --die-with-parent
          --proc /proc
          --dev /dev
          --clearenv
          --bind "$PRJ_ROOT" "$PRJ_ROOT"
          --bind "$PRJ_ROOT/target/tmp" /tmp
          --ro-bind "$DEVSHELL_DIR" "$DEVSHELL_DIR"
        ${
          if strictStore
          then
            lib.concatMapStringsSep "\n" (s: "  --ro-bind ${s}\n            ${s}")
            (lib.filter (
              s:
                lib.isString s
                && s != ""
                && s != pkgs.rustc.outPath
                && s != pkgs.rustc.unwrapped.outPath
            ) (lib.split "\n" (lib.readFile closure)))
          else "  --ro-bind /nix/store /nix/store"
        }
        )
        for k in $(env | cut -d= -f1); do
          case "$k" in
            CARGO|CARGO_*|ESP_*|OUT_DIR)
              bwrap_opts+=(--setenv "$k" "''${!k}")
              ;;
          esac
        done
        exec bwrap "''${bwrap_opts[@]}" -- ${unsafe}/bin/${lib.escapeShellArg exe} "$@"
      '';
    };

  # We don’t want `cargo` wrapper to add the original non-forked `rustc` to its `PATH`:
  cargo-any-rust = let
    original = pkgs.cargo;
  in
    pkgs.writeShellApplication {
      name = original.pname;
      meta.description = original.meta.description;
      text =
        lib.replaceStrings
        [original.passthru.rustc.outPath]
        [pkgs.emptyDirectory.outPath]
        (lib.readFile (lib.getExe original));
    };
in {
  packages = {
    inherit cargo-any-rust;
    bwrap-rustc = bubble-wrap-rust "rustc";
    bwrap-rustdoc = bubble-wrap-rust "rustdoc";
  };
}
