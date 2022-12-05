

registerProcessor("WasmProcessor", class WasmProcessor extends AudioWorkletProcessor {
  constructor(options) {
      super();
      let [module, memory, handle] = options.processorOptions;
      const bindgen = wasm_bindgen;
      console.log("bindgen.initSync");
      bindgen.initSync(module, memory);
      this.processor = bindgen.WasmAudioProcessor.unpack(handle);
  }
  process(inputs, outputs) {
    let insamples = inputs[0][0];
    if(insamples != null && insamples !== undefined) {
      return this.processor.process(insamples);
    }
  }
});