#!/bin/bash
set -e

# Build script for duckdb-pyfunc extension

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

BUILD_TYPE="${BUILD_TYPE:-Release}"
BUILD_DIR="${PROJECT_ROOT}/build"

echo "Building duckdb-pyfunc extension..."
echo "Build type: $BUILD_TYPE"
echo "Build directory: $BUILD_DIR"

# Create build directory
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

# Configure with CMake
cmake -DCMAKE_BUILD_TYPE="$BUILD_TYPE" ..

# Build
cmake --build . --config "$BUILD_TYPE" -j$(nproc)

echo ""
echo "Build complete!"
echo "Extension location: $BUILD_DIR/libpyfunc.so (or .dylib on macOS)"
