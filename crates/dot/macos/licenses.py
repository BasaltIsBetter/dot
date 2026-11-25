import json
import os
import pathlib
import subprocess
import sys

BINARIES = ("dot", "dot-shell", "dot-container")
TARGET = os.environ.get("DOT_TARGET", "aarch64-apple-darwin")
LICENSE_NAMES = ("license", "licence", "copying", "notice", "unlicense")
RULE = "=" * 79
MIT = """Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""


def metadata(manifest):
    command = [
        "cargo", "metadata", "--format-version", "1",
        "--filter-platform", TARGET, "--manifest-path", manifest,
    ]
    return json.loads(subprocess.run(command, check=True, capture_output=True, text=True).stdout)


def shipped(meta):
    packages = {package["id"]: package for package in meta["packages"]}
    nodes = {node["id"]: node for node in meta["resolve"]["nodes"]}
    waiting = [
        package["id"] for package in meta["packages"]
        if package["name"] in BINARIES and package["source"] is None
    ]
    seen = set()
    while waiting:
        package = waiting.pop()
        if package in seen:
            continue
        seen.add(package)
        for dependency in nodes[package]["deps"]:
            if any(kind["kind"] is None for kind in dependency["dep_kinds"]):
                waiting.append(dependency["pkg"])
    third_party = [packages[package] for package in seen if packages[package]["source"] is not None]
    return sorted(third_party, key=lambda package: (package["name"], package["version"]))


def license_files(package):
    folder = pathlib.Path(package["manifest_path"]).parent
    return sorted(
        path for path in folder.iterdir()
        if path.is_file() and path.name.lower().startswith(LICENSE_NAMES)
    )


def without_files(package):
    if "MIT" not in (package["license"] or ""):
        sys.exit(f"{package['name']} {package['version']} publishes no license file and offers no MIT license")
    holders = ", ".join(package["authors"]) or f"the authors of {package['name']}"
    return f"Used under the MIT License, one of the licenses it offers.\n\nCopyright (c) {holders}\n\n{MIT}"


def section(title, text):
    return f"{RULE}\n{title}\n{RULE}\n\n{text.rstrip()}\n\n"


def main():
    manifest, output, font_license, rtk_license = sys.argv[1:5]
    parts = [
        "Dot uses the following third-party components, each under its own license.\n\n",
        section("JetBrains Mono (font), SIL Open Font License 1.1", pathlib.Path(font_license).read_text()),
        section("RTK (rules adapted in dot-compact), Apache License 2.0", pathlib.Path(rtk_license).read_text()),
    ]
    for package in shipped(metadata(manifest)):
        title = f"{package['name']} {package['version']}, {package['license'] or 'no license declared'}"
        files = license_files(package)
        texts = [f"--- {path.name} ---\n{path.read_text(errors='replace').rstrip()}" for path in files]
        parts.append(section(title, "\n\n".join(texts) or without_files(package)))
    pathlib.Path(output).write_text("".join(parts))


main()
