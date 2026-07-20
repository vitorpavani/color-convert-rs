{
  description = "color-convert-rs — behavior-faithful Rust port of color-convert with CPU-SIMD and GPU acceleration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            # C toolchain — NixOS needs gcc + bfd linker for rustc to link binaries.
            # The rustup/system rustc references a broken ld-wrapper.sh; providing
            # gcc from nixpkgs and forcing the bfd linker via RUSTFLAGS fixes this.
            gcc
            binutils

            # JS/wasm toolchain for the npm drop-in replacement
            wasm-pack
            nodejs

            # GPU feature (optional — for --features gpu builds)
            vulkan-headers
            vulkan-loader
          ];

          # Force the bfd linker — the default lld wrapper bundled with rustc
          # references a nix store path that doesn't exist outside home-manager.
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=bfd";

          # Use the nix-provided gcc as the linker, bypassing the broken wrapper.
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.gcc}/bin/gcc";
        };
      });
}
