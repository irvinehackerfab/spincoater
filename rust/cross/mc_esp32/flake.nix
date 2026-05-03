{
  description = "ESP32-S3 Rust dev shell (espup + Nix)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # Rust toolchain installer
            rustup

            # Official esp-rs toolchain installer
            espup

            # Flashing / runner tools
            espflash
            ldproxy

            # Code generator
            esp-generate

            # Editor support
            rust-analyzer
          ];

          shellHook = ''
            echo "[ESP32 dev shell]"
            # --- Explicit toolchain exports (user-specific paths) ---
            # These are the exact exports you provided; we check existence first.
            ESP_RUSTUP_XTENSA_BIN="$HOME/.rustup/toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin"
            ESP_LIBCLANG="$HOME/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"

            if [ -d "$ESP_RUSTUP_XTENSA_BIN" ]; then
              export PATH="$ESP_RUSTUP_XTENSA_BIN:$PATH"
              echo "Prepended ESP xtensa toolchain bin: $ESP_RUSTUP_XTENSA_BIN"
            else
              echo "Warning: ESP xtensa toolchain bin not found at: $ESP_RUSTUP_XTENSA_BIN"
            fi

            if [ -d "$ESP_LIBCLANG" ]; then
              export LIBCLANG_PATH="$ESP_LIBCLANG"
              echo "Set LIBCLANG_PATH -> $ESP_LIBCLANG"
            else
              echo "Warning: esp-clang lib dir not found at: $ESP_LIBCLANG"
            fi

            # --- If rustup knows the 'esp' toolchain, prefer its bin dir too ---
            ESP_TOOLCHAIN_BIN="$(rustup which rustc --toolchain esp 2>/dev/null | xargs dirname || true)"
            if [ -n "$ESP_TOOLCHAIN_BIN" ] && [ -d "$ESP_TOOLCHAIN_BIN" ]; then
              export PATH="$ESP_TOOLCHAIN_BIN:$PATH"
              echo "Using esp toolchain rustc from: $ESP_TOOLCHAIN_BIN"
            fi

            # --- Informational ---
            if command -v espup >/dev/null 2>&1; then
              echo "espup available in the dev shell"
            else
              echo "Note: 'espup' not in PATH in this shell. You can still install the esp toolchain with 'espup install' once inside the dev shell."
            fi

            # Small helper: remind about CARGO_BUILD_TARGET
            echo "CARGO_BUILD_TARGET=$CARGO_BUILD_TARGET"
          '';
        };
      }
    );
}
