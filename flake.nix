{
  description = "Rust dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rust-overlay.overlays.default ];
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = [
        pkgs.rust-bin.nightly.latest.default
        pkgs.cargo
        pkgs.rust-analyzer
        pkgs.protobuf
        pkgs.openssl
        pkgs.pkg-config
      ];
    };
  };
}
