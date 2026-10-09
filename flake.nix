{
  description = "Terminal Markdown previewer with a GUI-like experience";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      overlays.default = final: _prev: {
        leaf-markdown-viewer = final.callPackage ./nix/package.nix { src = self; };
      };

      packages = forAllSystems (
        system:
        let
          leaf-markdown-viewer = (pkgsFor system).callPackage ./nix/package.nix { src = self; };
        in
        {
          inherit leaf-markdown-viewer;
          default = leaf-markdown-viewer;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              rust-analyzer
              rustc
              rustfmt
            ];
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-tree);
    };
}
