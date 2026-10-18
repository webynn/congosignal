// Mineral focus geometry: clustering into mining areas, outlines and the spotlight mask.
const {readFileSync}=require('node:fs');
const vm=require('node:vm'),assert=require('node:assert/strict');
const source=readFileSync('dashboard/app.js','utf8');
const data=JSON.parse(readFileSync('dashboard/geology/sites.geo.json','utf8'));
const ctx=vm.createContext({});
vm.runInContext(source.slice(source.indexOf('// focus-geometry:start'),source.indexOf('// focus-geometry:end')),ctx);
const site=(x,y,province='P')=>({geometry:{coordinates:[x,y]},properties:{province,name:'s',kind:'artisanal'}});

// ~0.15° of latitude is ~17 km: linked; ~0.27° is ~30 km: separate (20 km linkage).
assert.equal(vm.runInContext('clusterSites',ctx)([site(28,-2),site(28,-1.85)]).length,1);
assert.equal(vm.runInContext('clusterSites',ctx)([site(28,-2),site(28,-1.73)]).length,2);
// Chains link through intermediate sites.
assert.equal(vm.runInContext('clusterSites',ctx)([site(28,-2),site(28,-1.85),site(28,-1.7),site(28,-1.55)]).length,1);

// Areas need at least three sites and never span provinces or territories.
const t=(x,y,province,territory)=>{const s=site(x,y,province);s.properties.territory=territory;return s};
const areas=vm.runInContext('mineralAreas',ctx)([t(28,-2,'Sud-Kivu','Kalehe'),t(28.1,-2,'Sud-Kivu','Kalehe'),t(28.05,-1.92,'Sud-Kivu','Kalehe'),t(28.06,-1.95,'Nord-Kivu','Masisi'),t(28.07,-1.96,'Nord-Kivu','Masisi'),site(20,-8,'Kasaï')]);
assert.equal(areas.length,1,'two-site territory and isolated site are not areas');
const a=areas[0];
assert.equal(JSON.stringify(a.ring[0]),JSON.stringify(a.ring.at(-1)));
assert.equal(a.province,'Sud-Kivu');assert.equal(a.name,'Kalehe · Sud-Kivu');assert.equal(a.count,3);
const inside=(pt,ring)=>{let c=false;for(let i=0,j=ring.length-1;i<ring.length;j=i++){const[xi,yi]=ring[i],[xj,yj]=ring[j];if((yi>pt[1])!==(yj>pt[1])&&pt[0]<(xj-xi)*(pt[1]-yi)/(yj-yi)+xi)c=!c}return c};
for(const s of a.sites) assert(inside(s.geometry.coordinates,a.ring),'outline contains its sites');
assert(a.km2>50&&a.km2<5000,`plausible area ${a.km2}`);

// Mask holes wind opposite to the outer ring so the renderer cuts them out.
const mask=vm.runInContext('spotlightMask',ctx)(areas);
const sa=vm.runInContext('signedArea',ctx);
assert(Math.sign(sa(mask.geometry.coordinates[0]))!==Math.sign(sa(mask.geometry.coordinates[1])));

// Real data: gold forms several areas across several provinces, largest first.
const gold=data.features.filter(f=>f.properties.minerals.some(m=>m==='Or'||m==='Gold'));
const goldAreas=vm.runInContext('mineralAreas',ctx)(gold);
assert(goldAreas.length>=3,`gold areas ${goldAreas.length}`);
assert(new Set(goldAreas.map(x=>x.province)).size>=3,'gold spans several provinces');
assert(goldAreas.every(x=>x.km2<40000),'no region-sized blobs: '+Math.round(Math.max(...goldAreas.map(x=>x.km2))));
assert(goldAreas.every((x,i)=>i===0||goldAreas[i-1].count>=x.count),'sorted by site count');
assert(goldAreas.reduce((n,x)=>n+x.count,0)<=gold.length);
// Overlapping outlines merge into one hole; holes never overlap.
const near=vm.runInContext('mineralAreas',ctx)([t(28,-2,'Sud-Kivu','Kalehe'),t(28.05,-2,'Sud-Kivu','Kalehe'),t(28.02,-1.95,'Sud-Kivu','Kalehe'),t(28.06,-1.97,'Nord-Kivu','Masisi'),t(28.09,-1.96,'Nord-Kivu','Masisi'),t(28.08,-1.93,'Nord-Kivu','Masisi')]);
assert.equal(near.length,2);
assert.equal(vm.runInContext('spotlightMask',ctx)(near).geometry.coordinates.length,2,'two overlapping areas become one hole');
const goldMask=vm.runInContext('spotlightMask',ctx)(goldAreas).geometry.coordinates.slice(1);
const bb=r=>[Math.min(...r.map(q=>q[0])),Math.min(...r.map(q=>q[1])),Math.max(...r.map(q=>q[0])),Math.max(...r.map(q=>q[1]))];
for(let i=0;i<goldMask.length;i++)for(let j=i+1;j<goldMask.length;j++){const a=bb(goldMask[i]),b=bb(goldMask[j]);assert(!(a[0]<=b[2]&&b[0]<=a[2]&&a[1]<=b[3]&&b[1]<=a[3]),'mask holes do not overlap')}
console.log(`PASS: linkage clustering, minimum area size, closed containing outlines, mask winding, real gold data (${goldAreas.length} areas in ${new Set(goldAreas.map(x=>x.province)).size} provinces)`);
