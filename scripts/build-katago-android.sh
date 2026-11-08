#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NDK="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}"
if [[ -z "$NDK" || ! -d "$NDK" ]]; then
    echo "ANDROID_NDK_HOME must point to an Android NDK (r25b+)" >&2
    exit 1
fi

KATAGO_REF="${KATAGO_REF:-d91ea855110dae533f0aada947b2b7d78cc8a4e1}"
KATAGO_DIR="$ROOT/target/katago-src"
EIGEN_DIR="$ROOT/target/eigen"
DIST="$ROOT/target/katago-dist"
SYS="$NDK/toolchains/llvm/prebuilt/linux-x86_64/sysroot"
STRIP="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"

if [[ ! -d "$KATAGO_DIR/.git" ]]; then
    git clone https://github.com/lightvector/KataGo.git "$KATAGO_DIR"
fi
git -C "$KATAGO_DIR" fetch --depth 1 origin "$KATAGO_REF"
git -C "$KATAGO_DIR" checkout --force FETCH_HEAD

if [[ ! -d "$EIGEN_DIR/Eigen" ]]; then
    mkdir -p "$ROOT/target"
    curl -fsSL https://github.com/eigen-mirror/eigen/archive/refs/tags/3.4.0.tar.gz -o "$ROOT/target/eigen.tar.gz"
    rm -rf "$ROOT/target/eigen-3.4.0"
    tar -xzf "$ROOT/target/eigen.tar.gz" -C "$ROOT/target"
    mv "$ROOT/target/eigen-3.4.0" "$EIGEN_DIR"
fi

build() {
    local abi="$1"
    local android_abi="$2"
    local zlib="$SYS/usr/lib/$android_abi/29/libz.so"
    local build="$ROOT/target/katago-build-$abi"

    rm -rf "$build"
    cmake -S "$KATAGO_DIR/cpp" -B "$build" -G Ninja \
        -DCMAKE_TOOLCHAIN_FILE="$NDK/build/cmake/android.toolchain.cmake" \
        -DANDROID_ABI="$abi" \
        -DANDROID_PLATFORM=android-29 \
        -DANDROID_STL=c++_static \
        -DUSE_BACKEND=EIGEN \
        -DEIGEN3_INCLUDE_DIRS="$EIGEN_DIR" \
        -DZLIB_INCLUDE_DIR="$SYS/usr/include" \
        -DZLIB_LIBRARY="$zlib" \
        -DKATAGO_AUTO_FETCH_DEPS=OFF \
        -DBUILD_DISTRIBUTED=OFF \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_CXX_FLAGS="-DBYTE_ORDER=__BYTE_ORDER__ -DLITTLE_ENDIAN=__ORDER_LITTLE_ENDIAN__ -DBIG_ENDIAN=__ORDER_BIG_ENDIAN__"
    cmake --build "$build" --target katago -j"$(nproc)"

    mkdir -p "$DIST/$android_abi"
    cp "$build/katago" "$DIST/$android_abi/libkatago.so"
    "$STRIP" --strip-unneeded "$DIST/$android_abi/libkatago.so"
}

build arm64-v8a aarch64-linux-android
build x86_64 x86_64-linux-android

ls -lh "$DIST"/*/libkatago.so
