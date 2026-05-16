#!/usr/bin/env bash
set -euo pipefail

if command -v protoc >/dev/null 2>&1; then
    exec protoc "$@"
fi

version="27.3"
platform="linux-x86_64"
cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/nova-protoc"
cache_dir="$cache_root/$version/$platform"
protoc_bin="$cache_dir/bin/protoc"

if [[ ! -x "$protoc_bin" ]]; then
    mkdir -p "$cache_dir"
    tmp_dir="$(mktemp -d)"
    archive="$tmp_dir/protoc.zip"
    url="https://github.com/protocolbuffers/protobuf/releases/download/v${version}/protoc-${version}-${platform}.zip"

    curl -fsSL "$url" -o "$archive"
    unzip -q "$archive" -d "$cache_dir"
    rm -rf "$tmp_dir"
fi

exec "$protoc_bin" "$@"