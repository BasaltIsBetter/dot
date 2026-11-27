## Install

**Mac (Apple silicon)**: unzip `Dot-{version}-macos-arm64.zip`, then move `Dot.app` to Applications. This pre-release is not signed by Apple: the first time, macOS refuses to open it. To allow it, open System Settings › Privacy & Security › “Open Anyway”, or run once in a terminal:

```bash
xattr -dr com.apple.quarantine /Applications/Dot.app
```

macOS also asks again for keychain access the first time each new version connects.

**Linux (x86_64 or arm64, glibc 2.35 or newer, such as Ubuntu 22.04 and Debian 12)**: take the package of your processor (`uname -m`: `x86_64` or `aarch64`), then:

```bash
tar -xzf dot-{version}-x86_64-unknown-linux-gnu.tar.gz
./dot-{version}-x86_64-unknown-linux-gnu/install.sh
```

(`aarch64` instead of `x86_64` on arm64.)

Dot goes to `~/.local/share/dot-app`, the `dot` command to `~/.local/bin`, with its entry in the application launcher.

**Check a download**: `shasum -a 256 -c SHA256SUMS --ignore-missing`.

`Dot-{version}-macos-arm64.dSYM.zip` holds the debug symbols, to read the stack of a crash from the log: not needed to use Dot.

Dot speaks English, French, Spanish and Brazilian Portuguese; the “Language” setting changes it.
