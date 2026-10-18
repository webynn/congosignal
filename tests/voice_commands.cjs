const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const context=vm.createContext({});
vm.runInContext(fs.readFileSync('dashboard/voice.js','utf8').replaceAll('export function','function'),context);
const valid=context.supportedAction;
assert(valid({type:'navigate',target:'minerals',query:'Cobalt'}));
assert(valid({type:'search',query:'Goma'}));
assert(valid({type:'collect'}));assert(valid({type:'analyze'}));assert(valid({type:'none'}));
for(const action of [{type:'shell'},{type:'navigate',target:'https://evil.example',query:''},{type:'search',query:'x'.repeat(161)},{type:'search',query:''},null])assert.equal(valid(action),false);
console.log('PASS Okapi supported action boundaries and query limits');
