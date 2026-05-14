# Development Guide
This workspace contains a Nix dev shell for Rust. 

This dev environment eliminates the possibility of missing system libraries on your PC as this project progresses. If this dev environment is kept up to date, everyone who is using it will have tools that are up to date.

## Installation
1. Install [Nix](https://nixos.org/download/).
2. Install [direnv](https://direnv.net/), preferably with `nix-direnv`.
3. Enter the directory, and run `direnv allow`. From now on, the dev shell should activate automatically whenever you enter [this directory](./).

## Configuring your editor's rust-analyzer
The dev shell does not install any editors for you.

You can open this workspace in your own IDE, but rust-analyzer might not work unless your IDE has direnv integration.

If you'd like to use an editor without direnv support, you can find it in [nixpkgs](https://search.nixos.org/packages?channel=unstable) and add it to the flake.

### VSCode
Try [direnv-vscode](https://github.com/direnv/direnv-vscode).

### Vim/Neovim
Try [direnv.vim](https://github.com/direnv/direnv.vim).

### emacs
Try [emacs-direnv](https://github.com/wbolster/emacs-direnv).

### Zed
Zed has direnv support built-in, and the default setting has been tested to work.
