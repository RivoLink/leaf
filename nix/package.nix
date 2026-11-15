{
  lib,
  src ? lib.cleanSource ../.,
  installShellFiles,
  rustPlatform,
  versionCheckHook,
}:

rustPlatform.buildRustPackage {
  pname = "leaf-markdown-viewer";
  inherit ((lib.importTOML ../Cargo.toml).package) version;

  inherit src;

  __structuredAttrs = true;

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ installShellFiles ];

  # The config tests mutate the process-global LEAF_TAB_TITLE_LENGTH env var,
  # so they race against each other when cargo runs tests in parallel threads.
  dontUseCargoParallelTests = true;

  # `leaf --update` overwrites its own binary, which cannot work from the
  # read-only Nix store. Point at the Nix workflow instead. --replace-warn
  # keeps the build green if src/main.rs is refactored.
  postPatch = ''
    substituteInPlace src/main.rs \
      --replace-warn "mod update;" "#[allow(dead_code)] mod update;" \
      --replace-warn "use update::run_update;" "" \
      --replace-warn "        run_update()?;
            return Ok(());" '            bail!("leaf was installed through Nix; update it with your Nix configuration instead of `leaf --update`");'
  '';

  # Shipped as static files in completions/, so the built binary is never run
  # (keeps cross-compilation working).
  postInstall = ''
    installShellCompletion --cmd leaf \
      --bash completions/leaf.bash \
      --fish completions/leaf.fish \
      --nushell completions/leaf.nu \
      --zsh completions/leaf.zsh

    # installShellFiles has no PowerShell support and pwsh has no autoload
    # directory, so install by hand. Users dot-source it from their profile.
    install -Dm644 completions/leaf.ps1 $out/share/powershell/leaf.Completion.ps1
  '';

  doInstallCheck = true;
  nativeInstallCheckInputs = [ versionCheckHook ];
  versionCheckProgramArg = "--version";

  meta = {
    description = "Terminal Markdown previewer with a GUI-like experience";
    homepage = "https://leaf.rivolink.mg";
    license = lib.licenses.mit;
    mainProgram = "leaf";
    platforms = lib.platforms.unix;
  };
}
