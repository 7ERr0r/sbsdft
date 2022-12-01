class PortProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.port.onmessage = (event) => {
      console.log(event.data);
    };

    this.port.postMessage('Hi!');
  }

  process(inputs, outputs, parameters) {
    return true;
  }
}

registerProcessor('port-processor', PortProcessor);