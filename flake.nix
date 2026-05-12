{
  description = "leaf - A friendly terminal Markdown previewer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        craneLib = crane.mkLib pkgs;

        nativeBuildInputs = with pkgs; [ pkg-config ];

        # syntect's regex-onig feature requires the oniguruma C library.
        # reqwest uses rustls-tls (pure-Rust TLS) so no Security/SystemConfiguration
        # frameworks are needed. crossterm uses POSIX terminal APIs covered by the
        # stdenv SDK, so no explicit darwin framework inputs are required.
        buildInputs = with pkgs; [ oniguruma ];

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          inherit nativeBuildInputs buildInputs;
          # Tell the oniguruma-sys build script to link the system library.
          RUSTONIG_SYSTEM_LIBONIG = true;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        leaf = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          meta = with pkgs.lib; {
            description = "A friendly terminal Markdown previewer";
            homepage = "https://github.com/RivoLink/leaf";
            license = licenses.mit;
            mainProgram = "leaf";
            platforms = platforms.unix;
          };
        });
      in
      {
        packages = {
          default = leaf;
          inherit leaf;
        };

        apps.default = flake-utils.lib.mkApp { drv = leaf; };

        checks = {
          inherit leaf;

          leaf-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          leaf-fmt = craneLib.cargoFmt {
            src = commonArgs.src;
          };
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            clippy
            rustfmt
            cargo-edit
            cargo-watch
          ];
          RUSTONIG_SYSTEM_LIBONIG = true;
        };
      }
    );
}
