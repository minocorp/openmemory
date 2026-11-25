#!/bin/sh
set -eu

repo=minocorp/openmemory
dir=${OPENMEMORY_INSTALL_DIR:-$HOME/.local/bin}

case "$(uname -s)-$(uname -m)" in
Linux-x86_64 | Linux-amd64) target=x86_64-unknown-linux-musl ;;
Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-musl ;;
Darwin-x86_64) target=x86_64-apple-darwin ;;
Darwin-arm64 | Darwin-aarch64) target=aarch64-apple-darwin ;;
*)
  echo "openmemory: unsupported platform $(uname -s) $(uname -m)" >&2
  exit 1
  ;;
esac

if [ -n "${OPENMEMORY_DOWNLOAD_URL:-}" ]; then
  base=$OPENMEMORY_DOWNLOAD_URL
elif [ -n "${OPENMEMORY_VERSION:-}" ]; then
  base=https://github.com/$repo/releases/download/$OPENMEMORY_VERSION
else
  base=https://github.com/$repo/releases/latest/download
fi

if command -v sha256sum > /dev/null 2>&1; then
  sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum > /dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
  echo "openmemory: sha256sum or shasum is required" >&2
  exit 1
fi

asset=openmemory-$target.tar.gz
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$base/$asset" -o "$tmp/$asset"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"

expected=$(awk -v f="$asset" '$2 == f { print $1 }' "$tmp/SHA256SUMS")
if [ -z "$expected" ] || [ "$expected" != "$(sha256 "$tmp/$asset")" ]; then
  echo "openmemory: checksum mismatch for $asset" >&2
  exit 1
fi

tar -xzf "$tmp/$asset" -C "$tmp" openmemory
mkdir -p "$dir"
mv "$tmp/openmemory" "$dir/openmemory"
chmod 755 "$dir/openmemory"
if [ "$(uname -s)" = Darwin ]; then
  xattr -d com.apple.quarantine "$dir/openmemory" 2> /dev/null || true
fi

echo "openmemory installed to $dir/openmemory"
case ":$PATH:" in
*":$dir:"*) ;;
*) echo "Add $dir to your PATH: export PATH=\"$dir:\$PATH\"" ;;
esac
