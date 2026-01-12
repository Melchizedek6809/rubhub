{
  description = "RubHub";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      # Target system(s) — adjust if you need macOS or other
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
      {
        devShells = forAllSystems (system:
          let
            pkgs = nixpkgs.legacyPackages.${system};
          in
            {
              default = pkgs.mkShell {
                strictDeps = true;
                nativeBuildInputs = with pkgs; [
                  rustc
                  cargo
                  rustfmt
                  clippy
                  rust-analyzer
                  nodejs_24
                  typescript-language-server
                  bacon
                  biome
                  mold
                  clang
                  cargo-nextest
                ];

                RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
                LD = "${pkgs.mold}/bin/mold";
                CC = "${pkgs.llvmPackages.clang}/bin/clang";
                CXX = "${pkgs.llvmPackages.clang}/bin/clang++";
              };
            });
      };
}
