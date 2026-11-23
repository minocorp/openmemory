#!/bin/sh
set -eu

repo=minodevss/memory
dir=${MEMORY_INSTALL_DIR:-$HOME/.local/bin}

case "$(uname -s)-$(uname -m)" in
Linux-x86_64 | Linux-amd64) target=x86_64-unknown-linux-musl ;;
Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-musl ;;
Darwin-x86_64) target=x86_64-apple-darwin ;;
Darwin-arm64 | Darwin-aarch64) target=aarch64-apple-darwin ;;
*)
  echo "memory: unsupported platform $(uname -s) $(uname -m)" >&2
  exit 1
  ;;
esac

if [ -n "${MEMORY_DOWNLOAD_URL:-}" ]; then
  base=$MEMORY_DOWNLOAD_URL
elif [ -n "${MEMORY_VERSION:-}" ]; then
  base=https://github.com/$repo/releases/download/$MEMORY_VERSION
else
  base=https://github.com/$repo/releases/latest/download
fi

if command -v sha256sum > /dev/null 2>&1; then
  sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum > /dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
  echo "memory: sha256sum or shasum is required" >&2
  exit 1
fi

asset=memory-$target.tar.gz
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$base/$asset" -o "$tmp/$asset"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"

expected=$(awk -v f="$asset" '$2 == f { print $1 }' "$tmp/SHA256SUMS")
if [ -z "$expected" ] || [ "$expected" != "$(sha256 "$tmp/$asset")" ]; then
  echo "memory: checksum mismatch for $asset" >&2
  exit 1
fi

tar -xzf "$tmp/$asset" -C "$tmp" memory
mkdir -p "$dir"
mv "$tmp/memory" "$dir/memory"
chmod 755 "$dir/memory"
if [ "$(uname -s)" = Darwin ]; then
  xattr -d com.apple.quarantine "$dir/memory" 2> /dev/null || true
fi

echo "memory installed to $dir/memory"
case ":$PATH:" in
*":$dir:"*) ;;
*) echo "Add $dir to your PATH: export PATH=\"$dir:\$PATH\"" ;;
esac
