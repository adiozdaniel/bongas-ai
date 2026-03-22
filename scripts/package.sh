#!/bin/bash
set -e

VERSION="${1:-1.0.0}"
PLATFORM="linux-x64"
PACKAGE_NAME="bongas-ai-v${VERSION}-${PLATFORM}"

echo "=== Packaging BONGAS-AI v${VERSION} ==="

cd "$(dirname "$0")/.."

# Step 1: Build Rust binary
echo "[1/3] Building Rust release binary..."
cargo build --release

# Step 2: Assemble package
echo "[2/3] Assembling package..."
mkdir -p "dist/${PACKAGE_NAME}/models"

cp target/release/bongas-ai "dist/${PACKAGE_NAME}/"
cp -r models/*.safetensors "dist/${PACKAGE_NAME}/models/" 2>/dev/null || echo "  No .safetensors models found, skipping"
cp .env.example "dist/${PACKAGE_NAME}/" 2>/dev/null || true
cp config/default.toml "dist/${PACKAGE_NAME}/" 2>/dev/null || true

# Step 3: Create tarball
echo "[3/3] Creating tarball..."
cd dist
tar -czf "${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"
cd ..

echo "=== Package created: dist/${PACKAGE_NAME}.tar.gz ==="
echo "    Size: $(du -h "dist/${PACKAGE_NAME}.tar.gz" | cut -f1)"
