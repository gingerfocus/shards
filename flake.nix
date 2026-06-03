{
  inputs.nixpkgs.url = "nixpkgs";

  outputs = {
    self,
    nixpkgs,
  }: let
    lib = nixpkgs.lib;
    systems = [ "aarch64-linux" "x86_64-linux" ];
    eachSystem = f:
      lib.foldAttrs lib.mergeAttrs { }
      (map (s: lib.mapAttrs (_: v: { ${s} = v; }) (f s)) systems);
  in
    eachSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
      buildInputs = [];
      pname = "shards";
    in {
      # packages.default = pkgs.rustPlatform.buildRustPackage {
      #   inherit pname;
      #   version = "0.0.1";
      #
      #   cargoLock = {
      #     lockFile = ./Cargo.lock;
      #     outputHashes = {
      #       # "ext-0.1.0" = "sha256-50llOwQPEBNmEkDV6quVyNOkZFa78IV0a+eoxHqvVPA=";
      #     };
      #   };
      #
      #   src = ./.;
      #
      #   nativeBuildInputs = with pkgs; [ pkg-config 
      #   # rustup 
      #   # taplo 
      #   ];
      #   inherit buildInputs;
      #
      #   installPhase = let
      #     target = "target/${pkgs.stdenv.targetPlatform.config}/release";
      #   in ''
      #     install -Dm755 ${target}/${pname} $out/bin/${pname}
      #   '';
      #
      #   LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
      #
      #   meta = {
      #     maintainers = ["Evan Stokdyk <evan.stokdyk@gmail.com>"];
      #     description = "Tree Sitter Pre-Processor for Shards";
      #   };
      # };
    });
}
