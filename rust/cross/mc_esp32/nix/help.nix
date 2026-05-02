{
  config,
  lib,
  pkgs,
  ...
}: {
  packages.verify-bwrap = pkgs.writeShellApplication {
    name = "verify-bwrap";
    text = let
      red = builtins.fromJSON ''"\u001b[31m"'';
      reset = builtins.fromJSON ''"\u001b[0m"'';
      hint.linux = ''
        ${red}    error: Failed to run `bwrap […] -- rustc -V`.${reset}

        To use this devshell, you first need to set up a `bwrap` setuid wrapper to be
        able to use Bubblewrap sandboxing. If you're on NixOS, add the following
        fragment to your NixOS configuration:

          security.wrappers = {
            # Low-level unprivileged sandboxing tool, see <https://github.com/containers/bubblewrap>.
            bwrap = {
              owner = "root";
              group = "root";
              source = "''${pkgs.bubblewrap}/bin/bwrap";
              setuid = true;
            };
          };
      '';
      hint.darwin = ''
        ${red}    warning: on macOS, there is no sandboxing!${reset}

        Bubblewrap (`bwrap`) is only available on Linux. If you continue, you will run binary blobs
        from the internet directly on your machine.
      '';
      exe.linux = ''
        if ! ${lib.getExe config.packages.bwrap-rustc} --version | grep -qE '^rustc.*\(1.91.1.0\)$' ; then
          echo
          # shellcheck disable=SC2016
          cat <<<${lib.escapeShellArg (lib.trim hint.linux)}
          echo
          exit 1
        fi
      '';
      exe.darwin = ''
        # shellcheck disable=SC2016
        cat <<<${lib.escapeShellArg (lib.trim hint.darwin)}
        echo
      '';
    in
      if pkgs.stdenv.isLinux
      then exe.linux
      else exe.darwin;
  };
}
