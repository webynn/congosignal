// Execute the dashboard's actual pure/map-adapter functions without a browser.
const {readFileSync}=require('node:fs');
const vm=require('node:vm'),assert=require('node:assert/strict');
const source=readFileSync('dashboard/app.js','utf8');
const data=JSON.parse(readFileSync('dashboard/geology/sites.geo.json','utf8'));
const ctx=vm.createContext({state:{}, norm:s=>s.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toLowerCase(), L:a=>a, console, mapInstance:null});
vm.runInContext(source.slice(source.indexOf('state.geology = null;'),source.indexOf('function geologyArchive()')),ctx);
vm.runInContext(source.slice(source.indexOf('function mountMinerals()'),source.indexOf('function situationView()')),ctx);
ctx.state.geology=data;
assert.equal(vm.runInContext('mineralFeatures().length',ctx),3672);
ctx.state.mineralLayers=['IPIS'];assert.equal(vm.runInContext('mineralFeatures().length',ctx),3423);
ctx.state.mineralSearch='eohe';assert(vm.runInContext('mineralFeatures().every(f=>f.properties.name.toLowerCase().includes("eohe"))',ctx));
ctx.state.mineralSearch='';ctx.state.mineralLayers=['USGS'];ctx.state.mineralKind='deposit';
assert.equal(vm.runInContext('mineralFeatures().length',ctx),62);
ctx.state.mineralProvince='Lualaba';assert(vm.runInContext('mineralFeatures().every(f=>f.properties.province==="Lualaba")',ctx));
assert.equal(vm.runInContext('distanceKm([0,0],[0,0])',ctx),0);
assert(Math.abs(vm.runInContext('distanceKm([0,0],[1,0])',ctx)-111.195)<0.01);
ctx.state.nearbyRadius='25';ctx.situationItems=()=>[
 {id:'near',places:[{name:'near',lon:29,lat:-1}]},
 {id:'far',places:[{name:'far',lon:20,lat:-10}]},
 {id:'missing',places:[{name:'missing',lon:null,lat:null}]},
 {id:'twice',places:[{name:'a',lon:29,lat:-1},{name:'b',lon:29.01,lat:-1}]}];
assert.equal(vm.runInContext('nearbyMineralReports({geometry:{coordinates:[29,-1]}}).length',ctx),2);
const sources={},layers=[];let updates=0;
ctx.mapInstance={isStyleLoaded:()=>true,getSource:id=>sources[id],addSource:(id,v)=>sources[id]={...v,setData:d=>{updates++;sources[id].data=d}},addLayer:l=>layers.push(l)};
vm.runInContext('mountMinerals()',ctx);
assert.equal(sources['mineral-sites'].cluster,true);
assert(layers.some(l=>l.id==='mineral-clusters'));
vm.runInContext('mountMinerals(); mountMinerals()',ctx);assert.equal(updates,0,'idle must not loop by rewriting data');
ctx.state.mineralSelected=vm.runInContext('mineralFeatures()[0].id',ctx);
vm.runInContext('mountMinerals()',ctx);assert.equal(sources['mineral-selection'].data.features.length,1);
ctx.state.mineralSearch='no site matches this';vm.runInContext('mountMinerals()',ctx);
assert.equal(sources['mineral-sites'].data.features.length,0);assert.equal(sources['mineral-selection'].data.features.length,0);
for(const id of Object.keys(sources)) delete sources[id];
vm.runInContext('mountMinerals()',ctx);assert(sources['mineral-sites'],'style replacement restores source');
console.log('PASS: real-data filters, distance, unmapped exclusion, article deduplication, clustering, idle stability, selection clearing and style restoration');
