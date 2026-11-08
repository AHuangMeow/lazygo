# LazyGo

A small Android Go app built with Dioxus.

## Offline KataGo analysis

LazyGo bundles a stripped **KataGo v1.18.2 Eigen CPU** engine for `arm64-v8a`
(and `x86_64` for development). It does not bundle a neural network model.

1. Download a KataGo model ending in `.bin.gz`, such as
   [b6c96 or b10c128](https://katagoarchive.org/g170/neuralnets/README.txt).
2. Open **设置 → 模型 → 导入模型** and choose the file.
3. The model is validated once with KataGo, then stored in the app's private
   storage. Analysis is enabled after a valid model is imported.
4. Set the analysis depth in **设置 → 分析深度**. The default is 200 visits.

Analysis starts after every confirmed move and is cached by position, rules,
komi, depth, and model. The analysis panel is opened from the **分析** button
below the board. Undo and redo only move through analyzed positions; while
replaying, placing stones is disabled. Click any point in the chart to jump to
that turn.

The app is fully offline at runtime; model import uses the Android file picker
and does not download anything.

## Development

```sh
# Build the Android CPU engine into target/katago-dist
ANDROID_NDK_HOME="$ANDROID_HOME/ndk/29.0.13846066" \
  ./scripts/build-katago-android.sh

# Build and install the Android debug app
dx build --android
```

Set `KATAGO_ABIS=arm64-v8a` to build only the release ABI.

```sh
cargo test --lib
cargo clippy --all-targets
```

The release workflow builds the arm64 engine before assembling the signed APK
and verifies that `lib/arm64-v8a/libkatago.so` and `minSdkVersion:'29'` are
present.

## Licenses

LazyGo is MIT licensed. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for
KataGo, Eigen, zlib, and model notices.
