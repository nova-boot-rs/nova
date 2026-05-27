#!/usr/bin/env bash
set -euo pipefail

CRATES=(
    "nova-macros"
    "nova-resilience-store"
    "nova-boot"
    "nova-observability"
    "nova-boot-middleware"
    "nova-sql"
    "nova-nosql"
    "nova-graphdb"
    "nova-boot-messaging"
    "nova-boot-data-patterns"
)

set -a
source .env
set +a

echo "=== Log in to crates.io ==="
cargo login "$CRATES_IO_TOKEN" || {
    echo "FAILED: Unable to log in to crates.io"
    exit 1
}

echo "=== Publishing order check ==="
echo "Publishing in dependency order so internal crates are available before dependents."
for crate in "${CRATES[@]}"; do
    echo "Publishing $crate..."
    cargo publish -p "$crate"
    echo "Published $crate — waiting 30s..."
    sleep 30
done

echo ""
echo "✅ All crates published!"