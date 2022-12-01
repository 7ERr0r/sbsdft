Audio spectrum visualizer
===
Visualizes audio frequencies with DFT (Discrete Fourier Transform).



Program compilation
---

~~`cargo build --release`~~

`cargo install cargo-make`

Windows: `cargo make release`

Linux: `cargo make releasel`

Shader compilation
---
`make`


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
`cargo install cargo-make`
`cargo install wasm-bindgen-cli`
`cargo make rbo`