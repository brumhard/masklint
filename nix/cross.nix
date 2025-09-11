{
  crossSrc,
  pkgs,
}:
pkgs.rustPlatform.buildRustPackage {
  pname = "cargo-cross";
  version = "latest";

  src = crossSrc;

  checkFlags = [
    "--skip=docker::shared::tests::directories::test_host"

    # The following tests require empty CARGO_BUILD_TARGET env variable, but we
    # set it ever since https://github.com/NixOS/nixpkgs/pull/298108.
    "--skip=config::tests::test_config::no_env_and_no_toml_default_target_then_none"
    "--skip=config::tests::test_config::no_env_but_toml_default_target_then_use_toml"
  ];

  cargoHash = "sha256-wUWhWbnjKTPiAlc/c8TR7GM4nLhcO4ARZH+7sAc8BHo=";

  passthru = {
    updateScript = pkgs.nix-update-script {};
  };

  meta = {
    description = "Zero setup cross compilation and cross testing";
    homepage = "https://github.com/cross-rs/cross";
    license = with pkgs.lib.licenses; [
      asl20 # or
      mit
    ];
    mainProgram = "cross";
  };
}
