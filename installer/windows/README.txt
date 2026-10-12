Berserk CLI 1.0.0 for Windows x64

Install BerserkSetup-1.0.0-windows-x64.exe, then open a new PowerShell window:

  berserk --version
  berserk new app
  cd app
  berserk serve

The installer uses your Local AppData folder and adds its bin folder to your
user PATH. Administrator access is not required. Uninstall removes only the
PATH entry added by this installer and leaves projects elsewhere untouched.

The portable ZIP contains the same berserk.exe. Extract it and add that folder
to your user PATH, or invoke it by its full path.

To build and serve apps, install Rust 1.88 or later with the Windows MSVC
toolchain and Visual Studio C++ Build Tools (Desktop development with C++):
https://www.rust-lang.org/tools/install
https://visualstudio.microsoft.com/visual-cpp-build-tools/

This packaged CLI generates applications using the immutable Berserk v1.0.0
Git source, so pending crates.io publication does not prevent app builds.
Cargo requires internet access on the first build to fetch source/dependencies.

BUILD-MANIFEST.json records the CLI build source, framework source, toolchain,
and executable hash. The installer is unsigned; no code-signing identity is
claimed. SHA256SUMS verifies the release downloads.

Documentation: https://github.com/justinangeloperez327/berserk
License: MIT (see LICENSE)
