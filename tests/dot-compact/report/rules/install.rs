use super::*;

fn told(tool: &str, lines: &[&str]) -> Vec<Option<&'static str>> {
    (0..lines.len())
        .map(|at| {
            RULES
                .iter()
                .filter(|rule| rule.tools.contains(&tool))
                .find(|rule| rule.drops(lines, at))
                .map(|rule| rule.what)
        })
        .collect()
}

const INSTALL: Option<&str> = Some(INSTALL_PROGRESS);
const DEPRECATED: Option<&str> = Some(DEPRECATIONS);

#[test]
fn node_package_managers() {
    assert_eq!(
        told(
            "npm",
            &[
                "npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported",
                "npm WARN deprecated rimraf@3.0.2: old",
                "npm notice New major version of npm available! 10.0.0 -> 11.0.0",
                "added 312 packages, and audited 313 packages in 4s",
                "npm error code ERESOLVE"
            ]
        ),
        [DEPRECATED, DEPRECATED, INSTALL, None, None]
    );
    assert_eq!(
        told(
            "pnpm",
            &[
                "Progress: resolved 312, reused 300, downloaded 12, added 0",
                " WARN  deprecated glob@7.2.3: old",
                "Downloading registry.npmjs.org/typescript/5.4.5: 5.8 MB/5.8 MB, done",
                "Packages: +312",
                "Done in 4.2s"
            ]
        ),
        [INSTALL, DEPRECATED, INSTALL, None, None]
    );
    assert_eq!(
        told(
            "yarn",
            &[
                "➤ YN0013: │ lodash@npm:4.17.21 can't be found in the cache and will be fetched from the remote registry",
                "➤ YN0019: │ old-package-npm-1.0.0.zip appears to be unused - removing",
                "➤ YN0000: └ Completed in 2s",
                "➤ YN0002: │ app doesn't provide react"
            ]
        ),
        [INSTALL, INSTALL, None, None]
    );
}

#[test]
fn python_package_managers() {
    assert_eq!(
        told(
            "pip",
            &[
                "Requirement already satisfied: idna in ./venv/lib/python3.12/site-packages",
                "Collecting requests",
                "  Downloading requests-2.32.3-py3-none-any.whl (64 kB)",
                "  Using cached urllib3-2.2.2-py3-none-any.whl",
                "  Preparing metadata (pyproject.toml): started",
                "    Found existing installation: requests 2.31.0",
                "Installing collected packages: urllib3, requests",
                "Successfully installed requests-2.32.3 urllib3-2.2.2",
                "ERROR: No matching distribution found for nothing"
            ]
        ),
        [
            INSTALL, INSTALL, INSTALL, INSTALL, INSTALL, INSTALL, None, None, None
        ]
    );
    assert_eq!(
        told(
            "uv",
            &[
                "Downloading numpy (5.1MiB)",
                " Downloaded numpy",
                "   Building app @ file:///src",
                "      Built app @ file:///src",
                "Resolved 12 packages in 3ms",
                "Installed 2 packages in 4ms",
                " + numpy==2.0.0"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, INSTALL, None, None, None]
    );
    assert_eq!(
        told(
            "poetry",
            &[
                "  - Installing certifi (2024.7.4)",
                "  • Updating idna (3.6 -> 3.7)",
                "  - Removing six (1.16.0)",
                "Installing the current project: app (0.1.0)",
                "Package operations: 3 installs, 0 updates, 0 removals"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, None, None]
    );
}

#[test]
fn tool_version_managers_and_hook_environments() {
    assert_eq!(
        told(
            "mise",
            &[
                "mise Installing node@20.11.0",
                "mise Downloading node@20.11.0",
                "mise Extracting node@20.11.0",
                "mise node@20.11.0 installed",
                "mise ERROR no such tool"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, None, None]
    );
    assert_eq!(
        told(
            "pre-commit",
            &[
                "[INFO] Initializing environment for https://github.com/psf/black.",
                "[INFO] Installing environment for https://github.com/psf/black.",
                "[INFO] Once installed this environment will be reused.",
                "[INFO] This may take a few minutes...",
                "black....................................................................Passed"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, INSTALL, None]
    );
    assert_eq!(
        told(
            "uv",
            &[
                "Using cached numpy-2.0.0-cp312-cp312-macosx_14_0_arm64.whl",
                "   Preparing packages...",
                "Audited 12 packages in 3ms"
            ]
        ),
        [INSTALL, INSTALL, None]
    );
}

#[test]
fn ruby_php_and_apple_package_managers() {
    assert_eq!(
        told(
            "bundle",
            &[
                "Fetching gem metadata from https://rubygems.org/...",
                "Using rake 13.2.1",
                "Installing nokogiri 1.16.6 with native extensions",
                "Bundle complete! 12 Gemfile dependencies, 80 gems now installed."
            ]
        ),
        [INSTALL, INSTALL, INSTALL, None]
    );
    assert_eq!(
        told(
            "gem",
            &[
                "Parsing documentation for rake-13.2.1",
                "Installing ri documentation for rake-13.2.1",
                "Done installing documentation for rake after 0 seconds",
                "Successfully installed rake-13.2.1",
                "1 gem installed"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, None, None]
    );
    assert_eq!(
        told(
            "composer",
            &[
                "  - Downloading symfony/console (v7.1.2)",
                "  - Installing symfony/console (v7.1.2): Extracting archive",
                "  - Locking psr/log (3.0.0)",
                "Generating autoload files",
                "Package operations: 12 installs, 0 updates, 0 removals"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, None, None]
    );
    assert_eq!(
        told(
            "pod",
            &[
                "Installing Alamofire (5.9.1)",
                "Using SnapKit (5.7.1)",
                "Pod installation complete! There are 2 dependencies from the Podfile."
            ]
        ),
        [INSTALL, INSTALL, None]
    );
}

#[test]
fn system_package_managers() {
    assert_eq!(
        told(
            "brew",
            &[
                "==> Downloading https://ghcr.io/v2/homebrew/core/ripgrep/manifests/14.1.0",
                "==> Fetching ripgrep",
                "Already downloaded: /Users/me/Library/Caches/Homebrew/ripgrep.tar.gz",
                "==> Pouring ripgrep--14.1.0.arm64_sonoma.bottle.tar.gz",
                "🍺  /opt/homebrew/Cellar/ripgrep/14.1.0: 14 files, 6.1MB",
                "==> Caveats",
                "Warning: ripgrep 14.1.0 is already installed"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, INSTALL, None, None, None]
    );
    assert_eq!(
        told(
            "apt-get",
            &[
                "Get:1 http://deb.debian.org/debian bookworm InRelease [151 kB]",
                "Hit:2 http://deb.debian.org/debian bookworm-updates InRelease",
                "Reading package lists...",
                "Selecting previously unselected package curl.",
                "Unpacking curl (7.88.1-10) ...",
                "Setting up curl (7.88.1-10) ...",
                "The following NEW packages will be installed:",
                "E: Unable to locate package nothing"
            ]
        ),
        [
            INSTALL, INSTALL, INSTALL, INSTALL, INSTALL, INSTALL, None, None
        ]
    );
    assert_eq!(
        told(
            "apk",
            &[
                "fetch https://dl-cdn.alpinelinux.org/alpine/v3.20/main/aarch64/APKINDEX.tar.gz",
                "(1/12) Installing zlib (1.3.1-r1)",
                "(12/12) Upgrading busybox (1.36.1-r29)",
                "OK: 12 MiB in 30 packages",
                "(1/12) Something else",
                "(x) Installing"
            ]
        ),
        [INSTALL, INSTALL, INSTALL, None, None, None]
    );
    assert_eq!(
        told(
            "go",
            &[
                "go: downloading golang.org/x/text v0.16.0",
                "go: finding module for package x",
                "go: added golang.org/x/text v0.16.0",
                "ok  \texample.com/app\t0.2s"
            ]
        ),
        [INSTALL, INSTALL, None, None]
    );
}

#[test]
fn a_tool_only_has_its_own_rules() {
    assert_eq!(
        told(
            "npm",
            &[
                "Collecting requests",
                "Using rake 13.2.1",
                "Get:1 http://deb.debian.org"
            ]
        ),
        [None, None, None]
    );
    assert_eq!(
        told(
            "bundle",
            &[
                "  - Installing symfony/console (v7.1.2)",
                "Collecting requests"
            ]
        ),
        [None, None]
    );
}
