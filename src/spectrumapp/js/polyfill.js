if (!globalThis.TextDecoder) {
    globalThis.TextDecoder = class TextDecoder {
        decode(arg) {
            if (typeof arg !== 'undefined') {
                //throw Error('TextDecoder stub called');
                return String.fromCharCode.apply(String, arg);
            } else {
                return '';
            }
        }
    };
}

if (!globalThis.TextEncoder) {
    globalThis.TextEncoder = class TextEncoder {
        encode(arg) {
            if (typeof arg !== 'undefined') {
                //throw Error('TextEncoder stub called');
                let result = new Uint8Array(arg.length);
                for (let i = 0; i < arg.length; i++) {
                    result[i] = arg.charCodeAt(i);
                }
                return result;
            } else {
                return new Uint8Array(0);
            }
        }
    };
}

// export function polyfill_nop() {
// }