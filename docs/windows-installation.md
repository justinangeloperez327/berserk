# Install the Berserk CLI on Windows

Download and run [BerserkSetup-1.0.0-windows-x64.exe](https://github.com/justinangeloperez327/berserk/releases/download/v1.0.0/BerserkSetup-1.0.0-windows-x64.exe). The installer supports Windows 10/11 x64 and installs the CLI under `%LOCALAPPDATA%\Programs\Berserk` by default, adds its `bin` directory to your user PATH, and registers an uninstaller. Administrator rights are not required.

Open a **new PowerShell terminal** after installing:

~~~powershell
berserk --version
berserk new my-app
cd my-app
berserk serve
~~~

Visit `http://127.0.0.1:3000`.

## Requirements for compiling applications

The CLI can generate source files without Rust installed. Compiling and serving the generated project requires Rust **1.88 or later**, the MSVC toolchain, and the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/). Install Rust with [rustup](https://www.rust-lang.org/tools/install). The Berserk installer does not install these toolchains automatically.

Generated apps use an immutable Git-pinned version of the Berserk framework. The first build requires network access for source and Rust dependencies.

## Portable version and removal

The [v1.0.0 release](https://github.com/justinangeloperez327/berserk/releases/tag/v1.0.0) also provides `berserk-1.0.0-windows-x64.zip` and a standalone `berserk.exe`. You can run the executable directly or add its directory to PATH.

Remove the installed CLI through **Windows Settings → Installed apps → Berserk CLI**. Existing application projects are not removed.

The downloads are unsigned; verify file integrity with the release's `SHA256SUMS` asset. There is no claimed Microsoft code-signing identity.
