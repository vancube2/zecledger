#!/bin/sh
# ZecLedger installer.
#
#   curl -fsSL https://raw.githubusercontent.com/vancube2/zecledger/master/install.sh | sh
#
# Prefer to read it before you run it. That is a reasonable thing to want from a
# script that installs software which will read your viewing key:
#
#   curl -fsSL https://raw.githubusercontent.com/vancube2/zecledger/master/install.sh -o install.sh
#   less install.sh
#   sh install.sh
#
# What this does, and nothing else:
#   works out your platform, downloads the matching release from GitHub, checks it
#   against the published SHA256SUMS, verifies build provenance if you have the
#   GitHub CLI, and copies one binary onto your PATH.
#
# It never asks for a key, never sends anything anywhere, and installs from this
# repository only.

set -eu

REPO="vancube2/zecledger"
BIN="zecledger"

say() { printf '  %s\n' "$*"; }
die() {
    printf '\n  Error: %s\n\n' "$*" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || die "this script needs '$1', which was not found on your PATH."
}

need curl
need tar
need uname

# ---- which build do you need -------------------------------------------------

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
Linux)
    case "$arch" in
    x86_64 | amd64) target="x86_64-unknown-linux-gnu" ;;
    *) die "no prebuilt ZecLedger for Linux on $arch yet. You can build from source: https://github.com/$REPO" ;;
    esac
    ;;
Darwin)
    case "$arch" in
    arm64 | aarch64) target="aarch64-apple-darwin" ;;
    x86_64) target="x86_64-apple-darwin" ;;
    *) die "no prebuilt ZecLedger for macOS on $arch yet." ;;
    esac
    ;;
*)
    die "this script covers Linux and macOS. On Windows, download the zip from https://github.com/$REPO/releases/latest"
    ;;
esac

# checksum tool differs between the two
if command -v sha256sum >/dev/null 2>&1; then
    sha256() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
    sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    die "no sha256sum or shasum found, so the download cannot be verified. Refusing to install unverified software."
fi

printf '\n'
say "ZecLedger installer"
say "Platform: $os $arch  ->  $target"

# ---- which version -----------------------------------------------------------

say "Finding the latest release ..."
tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" |
    grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[^"]*"\([^"]*\)".*/\1/')"
[ -n "$tag" ] || die "could not work out the latest release. Check https://github.com/$REPO/releases"
version="${tag#v}"
say "Latest release: $tag"

archive="${BIN}-${version}-${target}.tar.gz"
base="https://github.com/$REPO/releases/download/$tag"

# ---- download ----------------------------------------------------------------

tmp="$(mktemp -d)"
# shellcheck disable=SC2064
trap "rm -rf '$tmp'" EXIT INT TERM

say "Downloading $archive ..."
curl -fsSL "$base/$archive" -o "$tmp/$archive" || die "could not download $base/$archive"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" || die "could not download SHA256SUMS, so the download cannot be verified."

# ---- verify ------------------------------------------------------------------

expected="$(grep " $archive\$" "$tmp/SHA256SUMS" | cut -d' ' -f1 | head -1)"
[ -n "$expected" ] || die "$archive is not listed in SHA256SUMS. Refusing to install."
actual="$(sha256 "$tmp/$archive")"

if [ "$expected" != "$actual" ]; then
    printf '\n'
    say "Checksum does NOT match."
    say "  expected: $expected"
    say "  actual:   $actual"
    die "refusing to install. Do not use this download. Please report it at https://github.com/$REPO/issues"
fi
say "Checksum verified."

# Provenance is a stronger check than a checksum: it proves the binary was built
# by the release workflow in this repository. Only possible with a recent gh.
if command -v gh >/dev/null 2>&1 && gh attestation --help >/dev/null 2>&1; then
    if gh attestation verify "$tmp/$archive" -R "$REPO" >/dev/null 2>&1; then
        say "Build provenance verified."
    else
        say "Note: could not verify build provenance. The checksum matched, so this"
        say "      is most likely gh not being signed in rather than a problem."
    fi
fi

# ---- unpack ------------------------------------------------------------------

tar -xzf "$tmp/$archive" -C "$tmp"
src="$tmp/${BIN}-${version}-${target}/${BIN}"
[ -f "$src" ] || die "the archive did not contain $BIN where expected."
chmod +x "$src"

# macOS marks anything downloaded as quarantined, and Gatekeeper then refuses to
# run it because the binary is not signed by a paid Apple developer account. The
# checksum and provenance above are a stronger guarantee than Apple's signature
# would be, so clear the flag rather than leave a working binary that will not run.
if [ "$os" = "Darwin" ]; then
    xattr -d com.apple.quarantine "$src" 2>/dev/null || true
fi

# ---- install -----------------------------------------------------------------

dest=""
if [ -w "/usr/local/bin" ]; then
    dest="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ] || mkdir -p "$HOME/.local/bin" 2>/dev/null; then
    dest="$HOME/.local/bin"
else
    die "nowhere writable to install to. Copy $src somewhere on your PATH yourself."
fi

mv "$src" "$dest/$BIN" || die "could not write to $dest"
say "Installed to $dest/$BIN"

# ---- check it actually runs --------------------------------------------------

if "$dest/$BIN" --version >/dev/null 2>&1; then
    say "Verified: $("$dest/$BIN" --version)"
else
    die "installed, but $dest/$BIN would not run."
fi

printf '\n'
case ":$PATH:" in
*":$dest:"*)
    say "Ready. Start with:"
    say "  zecledger sync"
    ;;
*)
    say "$dest is not on your PATH yet, so add this to your shell profile:"
    say "  export PATH=\"$dest:\$PATH\""
    say ""
    say "Then start with:"
    say "  zecledger sync"
    ;;
esac
printf '\n'
say "ZecLedger reads a viewing key. It never asks for a seed phrase or spending"
say "key and cannot move funds."
printf '\n'
