{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    nix2flatpak = {
      url = "github:neobrain/nix2flatpak";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs =
    inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import inputs.nixpkgs { inherit system; };
        lib = pkgs.lib;
        craneLib = inputs.crane.mkLib pkgs;

        cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
        name = cargoToml.package.name;

        compiletimeTools = with pkgs; [
          # for ffmpeg-sys-next
          rustPlatform.bindgenHook
          pkg-config
        ];
        compiletimeLibs = with pkgs; [
          ffmpeg
        ];

        runtimeExes = with pkgs; [
          ffmpeg
          mpv
        ];
        runtimeLibs = with pkgs; [
          # doesn't look like it's needed for some reason
          ffmpeg

          # needed for both x11 and wayland
          libxkbcommon

          wayland

          libx11
          libxcursor
          libxi
        ];

        commonArgs = {
          # all that's needed for artifacts and checks
          nativeBuildInputs = compiletimeTools;
          buildInputs = compiletimeLibs;

          src = craneLib.cleanCargoSource ./.;

          # workaround from https://crane.dev/faq/rebuilds-bindgen.html
          NIX_OUTPATH_USED_AS_RANDOM_SEED = "aaaaaaaaaa";
        };

        LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs;

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        crate =
          let
            desktopItem = pkgs.makeDesktopItem {
              inherit name;
              desktopName = name;
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
              exec = name;
            };
          in
          craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;

              nativeBuildInputs =
                with pkgs;
                commonArgs.nativeBuildInputs
                ++ [
                  makeBinaryWrapper
                ];
              buildInputs = commonArgs.buildInputs;

              postFixup = ''
                mkdir -p "$out/share/applications"
                ln -s "${desktopItem}"/share/applications/* "$out/share/applications/"

                wrapProgram $out/bin/${name} \
                  --prefix PATH : "${lib.makeBinPath runtimeExes}" \
                  --prefix LD_LIBRARY_PATH : "${LD_LIBRARY_PATH}"
              '';
            }
          );
      in
      {
        packages.default = crate;

        packages.flatpak = inputs.nix2flatpak.lib.${system}.mkFlatpak {
          developer = "electria";
          appId = "moe.pancake.${name}";
          package = crate;
          runtime = "org.gnome.Platform/49";
          permissions = {
            devices = [
              "dri"
            ];
            sockets = [
              "fallback-x11"
              "wayland"
            ];
          };
        };

        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit crate;

          # Run clippy on the crate source, resuing the dependency artifacts
          # (e.g. from build scripts or proc-macros) from above.
          #
          # Note that this is done as a separate derivation so it
          # does not impact building just the crate by itself.
          crate-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "-- --deny warnings";
            }
          );
        };

        devShells.default = craneLib.devShell {
          inherit LD_LIBRARY_PATH;

          inputsFrom = [ crate ];
          packages = [ pkgs.rust-analyzer ];
        };
      }
    );
}
