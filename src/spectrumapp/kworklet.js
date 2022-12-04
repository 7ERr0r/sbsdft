

registerProcessor("WasmProcessor", class WasmProcessor extends AudioWorkletProcessor {
  constructor(options) {
      super();
      let [module, memory, handle] = options.processorOptions;
      console.log("bindgen.initSync");
      bindgen.initSync(module, memory);
      this.processor = bindgen.WasmAudioProcessor.unpack(handle);
  }
  process(inputs, outputs) {
      return this.processor.process(inputs[0][0]);
  }
});


// let firstMsg = true;
// let donewbgmod = null;
// let preinitqueue = [];
// let wasmbindmodpromise = null;

// self.addEventListener('message', event => {
//     if (donewbgmod != null) {
//         // fastpath
//         donewbgmod.child_entry_point(event.data);
//     } else {
//         if (firstMsg) {
//             firstMsg = false;

//             (async () => {
//                 let wasmbindmod = await wasmbindmodpromise;

//                 console.log("onmessage_a", wasmbindmod);
//                 try {
//                     await wasmbindmod.default(...event.data);
//                     donewbgmod = wasmbindmod;

//                     for (let i = 0; i < preinitqueue.length; i++) {
//                         donewbgmod.child_entry_point(preinitqueue[i]);
//                     }
//                     preinitqueue = [];
//                 } catch (err) {
//                     setTimeout(() => {
//                         throw err;
//                     });
//                 }
//             })()
//         } else {
//             preinitqueue.push(event.data);
//         }
//     }
// });
// wasmbindmodpromise = import(wbgpath);



// // TODO

// class PortProcessor extends AudioWorkletProcessor {
//   constructor() {
//     super();
//     this.port.onmessage = (event) => {
//       console.log(event.data);
//     };

//     this.port.postMessage('Hi!');
//   }

//   process(inputs, outputs, parameters) {
//     return true;
//   }
// }

// registerProcessor('port-processor', PortProcessor);