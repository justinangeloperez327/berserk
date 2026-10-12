# Install Berserk on Windows

Download `BerserkSetup-1.0.0-windows-x64.exe` from the
[v1.0.0 release](https://github.com/justinangeloperez327/berserk/releases/tag/v1.0.0)
and run it. Windows 10/11 x64 is the tested target.

The installer uses `%LOCALAPPDATA%\Programs\Berserk` by default, adds its `bin`
folder to your user PATH, and registers an uninstaller. It requires no
administrator access. Open a **new terminal** after installation:

```powershell
berserk --version
berserk new app
cd app
berserk serve
```

Visit `http://127.0.0.1:3000`. The CLI can generate files without Rust installed;
building and serving an app requires Rust **1.88 or later** with its MSVC
toolchain and Visual Studio C++ Build Tools. Follow
[Rust installation](https://www.rust-lang.org/tools/install) and
[Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
The installer does not silently install either toolchain.

## Source and dependency selection

This packaged CLI embeds a full framework Git revision when it is built.
Generated apps point at the immutable v1.0.0 source in the official repository,
so they can compile before the coordinated crates.io publication. The first
Cargo build needs internet access for Git source and external dependencies.

Normal Cargo builds of the CLI keep registry dependencies as their default.
The packaging workflow explicitly sets `BERSERK_FRAMEWORK_GIT_REV`. Application
source generation remains centralized in `berserk-codegen`; the installer does
not rewrite project manifests or change the user's Cargo configuration.

`BUILD-MANIFEST.json` records the CLI packaging commit, pinned framework commit,
Rust toolchain, Cargo lockfile hash, and executable hash. The framework version
tag remains unchanged. The matching CLI source archive is included for rebuilding
the distributed executable with the recorded packaging configuration.

## Portable installation and removal

Alternatively, extract `berserk-1.0.0-windows-x64.zip` and add its folder to your
user PATH. Run the included executable directly if you prefer no PATH change.

Uninstall the setup distribution through Windows **Installed apps → Berserk
CLI**, or run its `unins000.exe`. The uninstaller removes only the PATH entry
the installer added. A pre-existing matching entry remains owned by the user.
Projects outside the installation folder are preserved.

The downloads are unsigned. Verify their SHA-256 digests against `SHA256SUMS`;
no Microsoft or other code-signing identity is claimed.

## Distribution verification

The Windows workflow tests installation in a path with spaces, CLI command
resolution and version output, repeated installation without duplicate PATH
entries, creation/build/serving of an independent Git-backed app, uninstall
preservation of unrelated PATH entries and projects, and a pre-existing PATH
entry. The full CI matrix also validates the packaging source before publication.
