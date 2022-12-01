Audio spectrum visualizer
===
Visualizes audio frequencies with DFT (Discrete Fourier Transform).



Program compilation
---

~~`cargo build --release`~~

`cargo install cargo-make`

Windows: `cargo make release`

Linux: `cargo make releasel`




Todo
---
- Organize code (less mutexes, separate crates)
- remove `cargo make`
- compute shaders instead of CPU DFT

Running
----
~~`cargo run --release -- -i stereo`~~



Name
---
Repo name is bad. Wanted to name it chromatone (similar to colorchord), but it's already taken: https://github.com/chromatone



Wasm compilation
---
- download https://github.com/WebAssembly/binaryen/releases/latest
- Windows: `wget https://github.com/WebAssembly/binaryen/releases/download/version_111/binaryen-version_111-x86_64-windows.tar.gz -O - | tar -zxvf - `
- Linux: `wget https://github.com/WebAssembly/binaryen/releases/download/version_111/binaryen-version_111-x86_64-linux.tar.gz -O - | tar -zxvf -`
- `mv binaryen-version_111 bin/binaryen`
- `cargo install cargo-make`
- `cargo install wasm-bindgen-cli`
- `npm install terser`
- `cargo make rbo`



Shader compilation
---

Unnecessary, because of new .wgsl
`make`