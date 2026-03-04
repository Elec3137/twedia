{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    crate2nix.url = "github:nix-community/crate2nix";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import inputs.nixpkgs { inherit system; };

        pname = (fromTOML (builtins.readFile ./Cargo.toml)).package.name;

        commonArgs = rec {
          nativeBuildInputs = with pkgs; [
            rustPlatform.bindgenHook
            makeBinaryWrapper
            pkg-config
          ];

          buildInputs = with pkgs; [
            ffmpeg

            libxkbcommon

            wayland

            libx11
            libxcursor
            libxi
          ];

          desktopItem = pkgs.makeDesktopItem {
            name = pname;
            desktopName = pname;
            mimeTypes = [
              "video/matroshka"
              "video/webm"
              "video/mp4"

              "audio/matroshka"
              "audio/webm"
              "audio/mp4"

              "audio/aac"
              "audio/flac"
              "audio/ogg"
            ];
            icon = "image-x-generic";
            exec = pname;
          };

          postFixup = ''
            mkdir -p "$out/share/applications"
            ln -s "${desktopItem}"/share/applications/* "$out/share/applications/"

            wrapProgram $out/bin/${pname} \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.ffmpeg ]} \
              --prefix LD_LIBRARY_PATH : ${LD_LIBRARY_PATH}
          '';
        };

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath commonArgs.buildInputs;

        cargoNix = inputs.crate2nix.tools.${system}.appliedCargoNix {
          name = pname;
          src = ./.;
        };
      in
      {
        packages.default = cargoNix.rootCrate.build.overrideAttrs commonArgs;

        checks = {
        };

        devShells.default = pkgs.mkShell commonArgs // {
          inherit LD_LIBRARY_PATH;

          packages = with pkgs; [
            rust-analyzer
            cargo
          ];
        };
      }
    );
}
