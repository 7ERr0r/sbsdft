

registerProcessor("WasmProcessor", class WasmProcessor extends AudioWorkletProcessor {
  constructor(options) {
      super();
      let [module, memory, handle] = options.processorOptions;

      // // legacy without modules
      // let wasmbinds = globalThis.wasm_bindgen;
      // if(!wasmbinds) {
      //   // modules
      //   wasmbinds = bindgen;
      // }
      console.log("wasmbinds.initSync");
      wasmbinds.initSync(module, memory);
      this.processor = wasmbinds.WasmAudioProcessor.unpack(handle);
  }
  process(inputs, outputs) {
    let insamples = inputs[0][0];
    if(insamples != null && insamples !== undefined) {
      return this.processor.process(insamples);
    }
  }
});