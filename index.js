let input = document.querySelector("#input");
let output = document.querySelector("#output");

function initialWasmRun() {
  var txt;
  try {
    txt = atob(window.location.hash.slice(1));
  } catch {
    console.log("couldn't decode url hash value");
  }

  if (txt.length > 0) {
    input.innerText = txt;
    wasmRun(txt);
  }
}

input.addEventListener('input', function(e) {
  window.location.hash = btoa(e.target.innerText);
  wasmRun(e.target.innerText);
});

var inputBts;
var outputBts;
var wasminst;

function wasmRun(txt) {
  inputBts = new TextEncoder().encode(txt);
  outputBts = "";
  try {
    wasminst.exports._start();
  } finally {
    output.innerText = outputBts;
  }
}

var importObject = {
  env: {
    "write": function(a, b) {
      s = new TextDecoder().decode(wasminst.exports.memory.buffer.slice(a, a+b));
      outputBts += s;
    },
    "write_err": function(a, b) {
      s = new TextDecoder().decode(wasminst.exports.memory.buffer.slice(a, a+b));
      console.log(s);
    },
    "read": function(a, b) {
      let n = Math.min(inputBts.byteLength, b);
      new Uint8Array(wasminst.exports.memory.buffer, a, b).set(inputBts.subarray(0, n));
      inputBts = inputBts.subarray(n);

      return n
    }
  },
};

WebAssembly.instantiateStreaming(
  fetch('main.wasm'),
  importObject
).then(result => {
  wasminst = result.instance;
  initialWasmRun();
});
