

#[cfg(not(target_arch = "wasm32"))]
pub mod spectrumapp;

#[cfg(not(target_arch = "wasm32"))]
pub fn main() {
    spectrumapp::main();
}

#[cfg(target_arch = "wasm32")]
pub fn main() {}
