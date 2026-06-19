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

        runtimeExes = with pkgs; [
          mpv
        ];
        dlDeps = with pkgs; [
          # needed for both x11 and wayland
          libxkbcommon

          wayland

          libx11
          libxcursor
          libxi
        ];

        commonArgs = {
          # all that's needed for artifacts and checks
          nativeBuildInputs = with pkgs; [
            # for ffmpeg-sys-next
            rustPlatform.bindgenHook
            pkg-config
          ];
          buildInputs = with pkgs; [
            ffmpeg
          ];

          src = craneLib.cleanCargoSource ./.;

          # workaround from https://crane.dev/faq/rebuilds-bindgen.html
          NIX_OUTPATH_USED_AS_RANDOM_SEED = "aaaaaaaaaa";
        };

        LD_LIBRARY_PATH = lib.makeLibraryPath dlDeps;

        # Build *just* the cargo dependencies,
        # to reuse them for build and test derivations.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself,
        # reusing the dependency artifacts from above.
        crate =
          let
            desktopItem = pkgs.makeDesktopItem {
              inherit name;
              desktopName = name;
              mimeTypes = cargoToml.package.metadata.bundle.linux_mime_types;
              icon = "image-x-generic";
              exec = name;
            };
          in
          craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;

              nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
                pkgs.makeBinaryWrapper
                pkgs.autoPatchelfHook
              ];

              runtimeDependencies = dlDeps;

              doCheck = false;

              postFixup = ''
                mkdir -p "$out/share/applications"
                ln -s "${desktopItem}"/share/applications/* "$out/share/applications/"

                wrapProgram $out/bin/${name} \
                  --prefix PATH : "${lib.makeBinPath runtimeExes}" \
              '';
            }
          );
      in
      {
        packages.default = crate;

        packages.flatpak = inputs.nix2flatpak.lib.${system}.mkFlatpak {
          developer = "electria";
          appId = cargoToml.package.metadata.bundle.identifier;
          package = crate;
          runtime = "org.gnome.Platform/49";
          permissions = {
            devices = [
              "dri"
            ];
            sockets = [
              "pulseaudio"
              "fallback-x11"
              "wayland"
            ];
          };
        };

        checks = {
          crate-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;

              cargoClippyExtraArgs = "-- --deny warnings && cargo test --release";

              env = {
                TESTFILE0 = pkgs.fetchurl {
                  url = "https://github.com/Elec3137/test-files/raw/refs/heads/main/chud.webm";
                  hash = "sha256-Z0p6mbJxWloCXzSongUs27XzLPCu9lPSbSSAYbwCHWg=";
                };
              };
            }
          );
        };

        devShells.default = craneLib.devShell {
          inherit LD_LIBRARY_PATH;

          inputsFrom = [ crate ];
          packages = [ pkgs.rust-analyzer ] ++ runtimeExes;
        };
      }
    );
}
