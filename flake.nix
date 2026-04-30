{
  description = "usbguard interface to list and allow devices with fuzzel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in
      {
        packages.default = rustPlatform.buildRustPackage {
          pname = "usbguard-fuzzel";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
          buildInputs = [ pkgs.usbguard pkgs.fuzzel ];
          postInstall = ''
            wrapProgram $out/bin/usbguard-fuzzel \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.usbguard pkgs.fuzzel ]}
          '';
        };

        checks.default = rustPlatform.buildRustPackage {
          pname = "usbguard-fuzzel-check";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          doCheck = true;
          checkPhase = ''
            cargo fmt --check
            cargo clippy -- -D warnings
            cargo test
          '';
          installPhase = ''
            mkdir -p $out
          '';
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ rustToolchain pkgs.pkg-config ];
          buildInputs = [ pkgs.usbguard pkgs.fuzzel ];
        };
      });
}
