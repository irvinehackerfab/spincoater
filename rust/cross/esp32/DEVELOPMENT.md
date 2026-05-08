# Development Guide
This workspace contains a Nix dev shell for Rust on ESP/ESP32. 

If this dev enviornment is kept up to date, everyone who is using it will have tools that are up to date. If you already set up Nix and direnv for host PC development, using this dev environment is less work than installing the tools yourself.

## Installation
1. Install [Nix](https://nixos.org/download/).
2. Install [direnv](https://direnv.net/), preferably with `nix-direnv`.
3. Enter the directory, and run `direnv allow`. From now on, the dev shell should activate automatically whenever you enter [esp32](./). 
4. Install the ESP32 toolchain with `espup install`.

## Configuring your editor's rust-analyzer
The dev shell does not install any editors for you.

You can open this workspace in your own IDE, but rust-analyzer will not work unless your IDE has direnv integration.

If you'd like to use an editor without direnv support, you can find it in [nixpkgs](https://search.nixos.org/packages?channel=unstable) and add it to the flake.

### VSCode
Try [direnv-vscode](https://github.com/direnv/direnv-vscode).

### Vim/Neovim
Try [direnv.vim](https://github.com/direnv/direnv.vim).

### emacs
Try [emacs-direnv](https://github.com/wbolster/emacs-direnv).

### Zed
Zed has direnv support built-in, and the default setting has been tested to work.
