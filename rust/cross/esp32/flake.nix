{
  description = "ESP32* Rust dev shell";

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
        rust-version = "1.98.1.0";
        suffix =
          {
            x86_64-linux = "x86_64-unknown-linux-gnu";
            aarch64-linux = "aarch64-unknown-linux-gnu";
            aarch64-darwin = "aarch64-apple-darwin";
          }
          .${system} or (throw "Unsupported system: ${system}");
        clean_suffix =
          {
            x86_64-linux = "x86_64-linux-gnu";
            aarch64-linux = "aarch64-linux-gnu";
            aarch64-darwin = suffix;
          }
          .${system};
        rust-src = pkgs.stdenv.mkDerivation rec {
          pname = "rust-src";
          version = rust-version;
          src = pkgs.fetchzip {
            url = "https://github.com/esp-rs/rust-build/releases/download/v${version}/${pname}-${version}.tar.xz";
            hash = "sha256-OSTa6kP4topLKZ13Tni6nQl6lNfeCj3kf4NFAzbrMzY=";
          };
          installPhase = ''
            bash ./install.sh --prefix=$out
          '';
        };
        rust = pkgs.stdenv.mkDerivation rec {
          pname = "rust";
          version = rust-version;
          src = pkgs.fetchzip {
            url = "https://github.com/esp-rs/rust-build/releases/download/v${version}/${pname}-${version}-${suffix}.tar.xz";
            hash =
              {
                x86_64-linux = "sha256-aBrU/6W0vnw49WvkAe+tfeDtn0xMPpwkKQ4ZPptLVbg=";
                aarch64-linux = pkgs.lib.fakeHash;
                aarch64-darwin = pkgs.lib.fakeHash;
              }
              .${system};
          };
          nativeBuildInputs = with pkgs; [ autoPatchelfHook ];
          buildInputs = with pkgs; [
            libgcc.lib
            libz
          ];
          installPhase = ''
            runHook preInstall
            bash ./install.sh --prefix=$out
            ln -s ${rust-src}/lib/rustlib/src $out/lib/rustlib/src
            runHook postInstall
          '';
        };
        xtensa-esp-elf = pkgs.stdenv.mkDerivation rec {
          pname = "xtensa-esp-elf";
          version = "16.1.0_20260609";
          src = pkgs.fetchzip {
            url = "https://github.com/espressif/crosstool-NG/releases/download/esp-${version}/${pname}-${version}-${clean_suffix}.tar.xz";
            hash =
              {
                x86_64-linux = "sha256-D02nz89injwvi+CD8tE8j/xkp1YrS28XvAmqCd3Dm+A=";
                aarch64-linux = "sha256-eF7iQr+9s6xOdk6F0vhxCcfF89pX10LamnxYwNCQYUk=";
                aarch64-darwin = "sha256-vk64HBLJu/bq15x1i/qBuObsmmlTcvlGrJJMcMMvJz0=";
              }
              .${system};
          };
          nativeBuildInputs = with pkgs; [ autoPatchelfHook ];
          buildInputs = with pkgs; [
            libgcc.lib
          ];
          installPhase = ''
            cp -r $src $out
          '';
        };
        clang-esp = pkgs.stdenv.mkDerivation rec {
          pname = "clang-esp";
          version = "21.1.3_20260408";
          src = pkgs.fetchzip {
            url = "https://github.com/espressif/llvm-project/releases/download/esp-${version}/${pname}-${version}-${clean_suffix}.tar.xz";
            hash =
              {
                x86_64-linux = "sha256-qtnyxsuZkggcprufxMyk6LzkZPgHTC6xbm8PTql9/WY=";
                aarch64-linux = "sha256-ol5URro6pLzPr6QyjOwkDFsa52OJeNhowmd3xM7KJBs=";
                aarch64-darwin = "sha256-sEo0XHtBJUAOTeeHNM7OARR+DMQ7V67vhG7zuP9X730=";
              }
              .${system};
          };
          nativeBuildInputs = with pkgs; [ autoPatchelfHook ];
          buildInputs = with pkgs; [ libgcc.lib ];
          installPhase = ''
            cp -r $src $out
          '';
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            # Flashing / runner tools
            pkgs.espflash
            # Editor support
            pkgs.rust-analyzer
            # ESP packages
            rust
            xtensa-esp-elf
            clang-esp
          ];

          LIBCLANG_PATH = "${xtensa-esp-elf}/lib";
        };
      }
    );
}
