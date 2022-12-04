Audio spectrum visualizer
===
Demo: [dzwi.ga/dft/](https://dzwi.ga/dft/)

Visualizes audio frequencies with DFT (Discrete Fourier Transform).

This app uses [Neighbour Components (NC) method](https://www.researchgate.net/publication/331834062_One_Technique_to_Enhance_the_Resolution_of_Discrete_Fourier_Transform).

Underlaying algorithm is Sb-SDFT [Single bin - sliding Discrete Fourier Transform](https://www.intechopen.com/chapters/54042).

Sliding DFT is calculated on the CPU using fixed-point integer math.


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
- remove statics with unsafe - use only `Arc`

Running
----
Windows: `cargo run --release -- -i stereo`

Linux: Make sure your default input device is 'Stereo Mix' - desktop audio monitor/loopback.

Then: `cargo run --release`


Name
---
Repo name is bad. Wanted to name it chromatone (similar to colorchord), but it's already taken: https://github.com/chromatone



Wasm compilation
---
- download https://github.com/WebAssembly/binaryen/releases/latest
- Linux: `wget https://github.com/WebAssembly/binaryen/releases/download/version_111/binaryen-version_111-x86_64-linux.tar.gz -O - | tar -zxvf -`
- `mv binaryen-version_111 bin/binaryen`
- `cargo install cargo-make`
- `cargo install wasm-bindgen-cli`
- `npm install terser`
- `cargo make rbo`



Shader compilation
---

Unnecessary, because of new .wgsl

Old SPIR-V compilation:
`make`