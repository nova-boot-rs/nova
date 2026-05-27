#!/usr/bin/env bash
set -euo pipefail

CRATES=(
    "nova-core"
    "nova-resilience-store"
    "nova-macros"
    "nova-observability"
    "nova-middleware"
    "nova-sql"
    "nova-nosql"
    "nova-graphdb"
    "nova-messaging"
    "nova-data-patterns"
)

set -a
source .env
set +a

echo "=== Log in to crates.io ==="
cargo login "$CRATES_IO_TOKEN" || {
    echo "FAILED: Unable to log in to crates.io"
    exit 1
}

echo "=== Dry-run checks ==="
for crate in "${CRATES[@]}"; do
    echo "Checking $crate..."
    cargo publish -p "$crate" --dry-run || {
        echo "FAILED: $crate"
        exit 1
    }
done

echo ""
echo "=== Publishing ==="
for crate in "${CRATES[@]}"; do
    echo "Publishing $crate..."
    cargo publish -p "$crate"
    echo "Published $crate — waiting 30s..."
    sleep 30
done

echo ""
echo "✅ All crates published!"