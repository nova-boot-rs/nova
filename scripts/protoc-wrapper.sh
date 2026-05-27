#!/usr/bin/env bash
set -euo pipefail

tmp_dir=""

cleanup() {
    if [[ -n "$tmp_dir" && -d "$tmp_dir" ]]; then
        rm -rf "$tmp_dir"
    fi
}

trap cleanup EXIT INT TERM

if command -v protoc >/dev/null 2>&1; then
    exec protoc "$@"
fi

version="27.3"
cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/nova-protoc"

detect_platform() {
    local uname_os uname_arch
    uname_os="$(uname -s)"
    uname_arch="$(uname -m)"

    case "$uname_os" in
        Linux)
            case "$uname_arch" in
                x86_64|amd64) echo "linux-x86_64:protoc" ;;
                aarch64|arm64) echo "linux-aarch_64:protoc" ;;
                *)
                    echo "unsupported Linux architecture: $uname_arch" >&2
                    return 1
                    ;;
            esac
            ;;
        Darwin)
            case "$uname_arch" in
                x86_64|amd64) echo "osx-x86_64:protoc" ;;
                arm64|aarch64) echo "osx-aarch_64:protoc" ;;
                *)
                    echo "unsupported macOS architecture: $uname_arch" >&2
                    return 1
                    ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT)
            case "$uname_arch" in
                x86_64|amd64) echo "win64:protoc.exe" ;;
                i386|i686|x86) echo "win32:protoc.exe" ;;
                *)
                    echo "unsupported Windows architecture: $uname_arch" >&2
                    return 1
                    ;;
            esac
            ;;
        *)
            echo "unsupported operating system: $uname_os" >&2
            return 1
            ;;
    esac
}

platform_info="$(detect_platform)"
platform="${platform_info%%:*}"
protoc_exe_name="${platform_info##*:}"
cache_dir="$cache_root/$version/$platform"
protoc_bin="$cache_dir/bin/$protoc_exe_name"
checksum_file="$cache_dir/.sha256"
expected_checksum="$(expected_sha256)"

expected_sha256() {
    case "$platform" in
        linux-x86_64) echo "6dab2adab83f915126cab53540d48957c40e9e9023969c3e84d44bfb936c7741" ;;
        linux-aarch_64) echo "bdad36f3ad7472281d90568c4956ea2e203c216e0de005c6bd486f1920f2751c" ;;
        osx-x86_64) echo "ce282648fed0e7fbd6237d606dc9ec168dd2c1863889b04efa0b19c47da65d1b" ;;
        osx-aarch_64) echo "b22116bd97cdbd7ea25346abe635a9df268515fe5ef5afa93cd9a68fc2513f84" ;;
        win64) echo "bdccd341a0c25ff4f65d8530f170c78ce5681e42920aa5d892f27ac06cc3bba0" ;;
        win32) echo "e22c4084111953d17c92effb004ed06e9e4516022950e12ee7bc3af0c4497326" ;;
        *)
            echo "unsupported platform: $platform" >&2
            return 1
            ;;
    esac
}

archive_sha256() {
    local archive_path="$1"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$archive_path" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$archive_path" | awk '{print $1}'
    else
        echo "missing sha256 tool (sha256sum or shasum)" >&2
        return 1
    fi
}

if [[ -x "$protoc_bin" && -f "$checksum_file" && "$(<"$checksum_file")" == "$expected_checksum" ]]; then
    exec "$protoc_bin" "$@"
fi

rm -rf "$cache_dir"
mkdir -p "$cache_dir"
tmp_dir="$(mktemp -d)"
archive="$tmp_dir/protoc.zip"
url="https://github.com/protocolbuffers/protobuf/releases/download/v${version}/protoc-${version}-${platform}.zip"

curl -fsSL "$url" -o "$archive"

downloaded_sha256="$(archive_sha256 "$archive")"
if [[ "$downloaded_sha256" != "$expected_checksum" ]]; then
    echo "checksum verification failed for protoc-${version}-${platform}.zip" >&2
    echo "expected: $expected_checksum" >&2
    echo "actual:   $downloaded_sha256" >&2
    exit 1
fi

unzip -q "$archive" -d "$cache_dir"
printf '%s\n' "$expected_checksum" > "$checksum_file"

cleanup

exec "$protoc_bin" "$@"