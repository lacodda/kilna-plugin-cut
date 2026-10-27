#!/bin/sh
# kilna-plugin-cut installer for macOS and Linux:
#   curl -fsSL https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.sh | sh
set -eu

REPO="lacodda/kilna-plugin-cut"

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) TARGET="x86_64-unknown-linux-gnu"; HOME_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/com.lacodda.kilna" ;;
    Darwin-arm64) TARGET="aarch64-apple-darwin"; HOME_DIR="$HOME/Library/Application Support/com.lacodda.kilna" ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "This is the macOS/Linux installer, but you are on Windows ($(uname -s))." >&2
        echo "Install with PowerShell instead:" >&2
        echo "  irm https://raw.githubusercontent.com/$REPO/main/tools/install.ps1 | iex" >&2
        exit 1
        ;;
    *)
        echo "No prebuilt binary for $(uname -s)/$(uname -m); build it with: cargo install --git https://github.com/$REPO" >&2
        exit 1
        ;;
esac

# The tag comes from the /releases/latest redirect rather than the REST API:
# unauthenticated API calls are capped at 60 per hour per IP.
# KILNA_PLUGIN_CUT_VERSION pins a specific release.
TAG="${KILNA_PLUGIN_CUT_VERSION:-}"
if [ -z "$TAG" ]; then
    LOCATION=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" || true)
    TAG="${LOCATION##*/}"
fi
case "$TAG" in
    v[0-9]*) ;;
    *)
        echo "Cannot resolve the latest release of $REPO - set KILNA_PLUGIN_CUT_VERSION to a tag like v0.1.0" >&2
        exit 1
        ;;
esac

NAME="kilna-plugin-cut-$TAG-$TARGET"
URL="https://github.com/$REPO/releases/download/$TAG/$NAME.tar.gz"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Downloading $URL"
curl -fsSL "$URL" | tar xz -C "$TMP"
BIN=$(find "$TMP" -type f -name kilna-plugin-cut | head -n 1)
[ -n "$BIN" ] || { echo "The release archive holds no kilna-plugin-cut" >&2; exit 1; }

# kilna looks for plugins in the plugins folder beside its workspace.
DIR="${KILNA_PLUGIN_CUT_DIR:-$HOME_DIR/plugins}"
mkdir -p "$DIR"
install -m 755 "$BIN" "$DIR/kilna-plugin-cut"
echo "Installed kilna-plugin-cut $TAG to $DIR/kilna-plugin-cut"

# The settings stay as the person wrote them: an example only when there is none.
if [ ! -f "$DIR/kilna-plugin-cut.json" ]; then
    printf '{\n  "channel": "",\n  "call": "FULL VERSION",\n  "cover_seconds": 0.5\n}\n' > "$DIR/kilna-plugin-cut.json"
    echo "Wrote $DIR/kilna-plugin-cut.json - put your channel's name in it."
fi

command -v ffmpeg >/dev/null 2>&1 || echo "Note: ffmpeg is not on your PATH; the plugin needs it."
echo "In kilna, 'Cut the short' now stands on a short's Cuts tab."
