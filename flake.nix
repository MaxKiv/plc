{
  description = "Development tooling for stm32";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    ...
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = (import nixpkgs) {
        inherit system;
      };

      # Get a cross compilation toolchain from the rust-toolchain.toml
      toolchain = with fenix.packages.${system};
        fromToolchainFile {
          file = ./rust-toolchain.toml; # alternatively, dir = ./.;
          sha256 = "sha256-WInuoEKQQy+RPFLGMp0sLDxCzFKfiixAE6s1ssLYxcc=";
        };
    in {
      # Development shells provided by this flake, to use:
      # nix develop .#default
      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          nil # Nix LSP
          alejandra # Nix Formatter
          toolchain # Our Rust toolchain
          rust-analyzer # Rust LSP
          probe-rs-tools # probe-rs
          gcc-arm-embedded # arm-none-eabi-gdb
          openocd # gdb server
          tio # serial monitor
        ];
      };

      formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.alejandra;
    });
}
