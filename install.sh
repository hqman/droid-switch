#!/usr/bin/env sh
set -eu

repo="${DSW_REPO:-hqman/droid-switch}"
version="${DSW_VERSION:-latest}"
install_dir="${DSW_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin)
    case "$arch" in
      arm64|aarch64) target="aarch64-apple-darwin" ;;
      x86_64|amd64)
        echo "unsupported macOS arch: $arch (prebuilt Intel macOS release is not available yet)" >&2
        exit 1
        ;;
      *) echo "unsupported macOS arch: $arch" >&2; exit 1 ;;
    esac
    archive_ext="tar.gz"
    ;;
  Linux)
    case "$arch" in
      x86_64|amd64) target="x86_64-unknown-linux-gnu" ;;
      *) echo "unsupported Linux arch: $arch" >&2; exit 1 ;;
    esac
    archive_ext="tar.gz"
    ;;
  *)
    echo "unsupported OS: $os" >&2
    exit 1
    ;;
esac

name="dsw"
asset="$name"
if [ "$version" = "latest" ]; then
  url="https://github.com/$repo/releases/latest/download/$asset-$target.$archive_ext"
else
  url="https://github.com/$repo/releases/download/$version/$asset-$target.$archive_ext"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "downloading $url"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$tmp/$asset.$archive_ext"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$url" -O "$tmp/$asset.$archive_ext"
else
  echo "curl or wget is required" >&2
  exit 1
fi

tar -xzf "$tmp/$asset.$archive_ext" -C "$tmp"
mkdir -p "$install_dir"
find "$tmp" -type f -name dsw -exec cp {} "$install_dir/dsw" \;
chmod 755 "$install_dir/dsw"

echo "installed: $install_dir/dsw"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) echo "note: add $install_dir to PATH if dsw is not found" ;;
esac
