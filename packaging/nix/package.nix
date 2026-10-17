{
  lib,
  rustPlatform,
  qt6,
  lld,
  autoPatchelfHook,
  stdenv,
  libGL,
}:
let
  # cxx-qt-build asks qmake for every Qt module's .prl; nixpkgs splits Qt into
  # one store path per module, so qmake needs a merged view to find QtQml.
  qt = qt6.env "qt6-omarchy-cheatsheets" [ qt6.qtdeclarative ];
in
rustPlatform.buildRustPackage {
  pname = "omarchy-cheatsheets";
  version = (lib.importTOML ../../Cargo.toml).package.version;

  src = lib.cleanSource ../..;
  cargoLock.lockFile = ../../Cargo.lock;

  nativeBuildInputs = [
    qt6.wrapQtAppsHook
    lld
    autoPatchelfHook
  ];
  buildInputs = [
    stdenv.cc.cc.lib
    libGL
    qt6.qtbase
    qt6.qtdeclarative
    qt6.qtwayland
  ];

  preBuild = "export QMAKE=${qt}/bin/qmake";
  # With GNU gold the cxx-qt static initializers end up undefined at link
  # time; lld links them. lld skips the nixpkgs ld wrapper, so
  # autoPatchelfHook restores the runtime library paths.
  RUSTFLAGS = "-C link-arg=-fuse-ld=lld";
  # The test suite needs a display for the QML smoke tests.
  doCheck = false;

  postInstall = ''
    install -Dm644 packaging/omarchy-cheatsheets.desktop -t $out/share/applications
  '';

  meta = {
    description = "Pretty, searchable cheatsheets for Omarchy Linux";
    homepage = "https://github.com/akitaonrails/omarchy-cheatsheets";
    license = lib.licenses.gpl3Only;
    mainProgram = "omarchy-cheatsheets";
    platforms = lib.platforms.linux;
  };
}
