# Third-party notices

LazyGo bundles a CPU build of KataGo and links against the following components.

## KataGo

Copyright 2025 David J Wu ("lightvector") and/or other authors of the content in
the KataGo repository. See the KataGo `CONTRIBUTORS` file for the full list.

KataGo is distributed under a permissive MIT-style license:

> Permission is hereby granted, free of charge, to any person obtaining a copy of
> this software and associated documentation files (the "Software"), to deal in
> the Software without restriction, including without limitation the rights to
> use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
> of the Software, and to permit persons to whom the Software is furnished to do
> so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

The KataGo repository also contains third-party components. The CPU-only build
used by LazyGo includes Eigen and links the Android platform zlib. See the
KataGo `LICENSE` file and `cpp/external` for the complete dependency notices.

- KataGo: https://github.com/lightvector/KataGo
- Eigen: https://eigen.tuxfamily.org/ (MPL2)
- zlib: https://zlib.net/ (zlib license)
- Android NDK libc++: https://developer.android.com/ndk (Apache-2.0 with LLVM exceptions)

## Neural network models

LazyGo does **not** bundle a KataGo model. Models are downloaded and imported by
the user. Their licenses and terms are separate from the LazyGo and KataGo code
licenses; users are responsible for checking the model source they import.
