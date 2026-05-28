#!/usr/bin/env bash
set -euo pipefail

CRATES=(
    "nova-boot-macros"
    "nova-boot-resilience-store"
    "nova-boot"
    "nova-boot-observability"
    "nova-boot-middleware"
    "nova-boot-sql"
    "nova-boot-nosql"
    "nova-boot-graphdb"
    "nova-boot-messaging"
    "nova-boot-data-patterns"
    "nova-boot-discovery-consul"
    "nova-boot-discovery-etcd"
    "nova-boot-discovery-dns"
    "nova-boot-client"
    "nova-boot-test"
    "nova-boot-gateway"
    "nova-boot-auth"
    "nova-boot-cli"
    "nova-boot-tasks"
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