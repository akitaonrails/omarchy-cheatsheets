{
  description = "Pretty, searchable cheatsheets for Omarchy Linux";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: {
        default = pkgs.callPackage ./packaging/nix/package.nix { };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = with pkgs; [
            clippy
            rustfmt
            cargo-nextest
            cargo-deny
          ];
          shellHook = "export QMAKE=${pkgs.qt6.env "qt6-omarchy-cheatsheets" [ pkgs.qt6.qtdeclarative ]}/bin/qmake";
          # lld (see package.nix) skips the ld wrapper's rpaths, so test binaries need the libs here.
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.qt6.qtbase
            pkgs.qt6.qtdeclarative
            pkgs.libGL
            pkgs.stdenv.cc.cc.lib
          ];
        };
      });
    };
}
