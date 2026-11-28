// Fails when the browser core uses a WebAssembly encoding or feature that a
// stock engine does not accept. Node ships a V8 older than current browsers,
// so a module it validates also loads in the browsers refl supports.
import { readFileSync } from "node:fs";

const path = process.argv[2] ?? "src/generated/wasm/refl_core_wasm_bg.wasm";
const bytes = readFileSync(path);
if (!WebAssembly.validate(bytes)) {
  console.error(`${path}: WebAssembly.validate rejected the module (an experimental feature or encoding slipped in)`);
  process.exit(1);
}
console.log(`${path}: valid for a stock engine (${bytes.length} bytes)`);
