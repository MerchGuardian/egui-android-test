{
  description = "Rust flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
          ];
          config = {
            # Allow android studio / sdk's
            allowUnfree = true;
            android_sdk.accept_license = true;
          };
        };
        rust-pkgs = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-analyzer" "rust-src" ];
          targets = [ "aarch64-linux-android" ];
        };
        ndk-version = "25.1.8937393";
        android-pkgs = pkgs.androidenv.composeAndroidPackages {
          buildToolsVersions = [ "33.0.0" ];
          includeNDK = true;
          includeSources = true;
          ndkVersions = [ ndk-version ];
          cmakeVersions = [ "3.22.1" ];
          platformVersions = [ "33" ];
          abiVersions = [ "arm64-v8a" ];
        };
        android-sdk = android-pkgs.androidsdk;
      in {
        devShells.default = pkgs.mkShell {
          CARGO_NET_GIT_FETCH_WITH_CLI = "true";

          packages = with pkgs; [
            rust-pkgs
            android-sdk
            cargo-ndk
            gradle_8
            jdk22
          ];

          shellHook = ''
            export ANDROID_HOME=${android-sdk}/libexec/android-sdk;
            export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/${ndk-version}/;
            export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/${ndk-version}/;
            rm -f .androidsdk && ln -s $ANDROID_HOME .androidsdk
          '';
        };
      });
}
