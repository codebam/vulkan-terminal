{
  description = "Rust Flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable-small";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          rustc
          rust-analyzer
          rustfmt
          pkg-config

          # Vulkan development
          vulkan-headers
          vulkan-loader
          vulkan-tools
          vulkan-validation-layers
          shaderc

          # Graphics and windowing
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi

          # Build dependencies
          cmake
          gcc
          gnumake

          # Font for the terminal
          dejavu_fonts
        ];

        shellHook = ''
          export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d"
          export LD_LIBRARY_PATH="${pkgs.vulkan-loader}/lib:${pkgs.libxkbcommon}/lib:${pkgs.wayland}/lib:$LD_LIBRARY_PATH"
          export VULKAN_SDK="${pkgs.vulkan-headers}"

          # Copy font to assets if it doesn't exist
          mkdir -p assets
          if [ ! -f assets/DejaVuSansMono.ttf ]; then
            cp "${pkgs.dejavu_fonts}/share/fonts/truetype/DejaVuSansMono.ttf" assets/ 2>/dev/null || echo "Font not found, please add DejaVuSansMono.ttf to assets/"
          fi

          echo "Vulkan development environment loaded"
          echo "Vulkan tools available: vulkaninfo, vkcube"
          echo "Shader compiler: glslc"
          echo "To compile shaders: glslc shaders/text.vert -o shaders/text.vert.spv && glslc shaders/text.frag -o shaders/text.frag.spv"
        '';
      };
    };
}
