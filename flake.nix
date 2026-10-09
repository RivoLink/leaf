{
  description = "Terminal Markdown previewer with a GUI-like experience";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "leaf-markdown-viewer";
          inherit version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.installShellFiles ];
          postInstall = ''
            installShellCompletion \
              --bash completions/leaf.bash \
              --fish completions/leaf.fish \
              --zsh completions/leaf.zsh
          '';
          meta = with pkgs.lib; {
            description = "Terminal Markdown previewer with a GUI-like experience";
            homepage = "https://github.com/RivoLink/leaf";
            license = licenses.mit;
            mainProgram = "leaf";
          };
        };
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ rustc cargo rust-analyzer clippy rustfmt ];
        };
      });
}
