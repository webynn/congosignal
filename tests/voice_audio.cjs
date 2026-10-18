const assert=require('node:assert/strict');
(async()=>{
 const fs=require('node:fs');const src=fs.readFileSync('dashboard/voice.js','utf8');
 const {speechText,pcm16Samples}=await import('data:text/javascript;base64,'+Buffer.from(src).toString('base64'));
 assert.equal(speechText('**Hello** [report:abc123]. Read [the source](https://example.org).'),'Hello . Read the source.');
 assert.equal(speechText('Copper: 2,600 tonnes. [mineral:USGS-12]'),'Copper: 2,600 tonnes.');
 assert.deepEqual([...pcm16Samples(Uint8Array.from([0,0,0,128,255,127]))],[0,-1,32767/32768]);
 assert.throws(()=>pcm16Samples(Uint8Array.from([0])));
 console.log('PASS speech markup cleaning and signed little-endian PCM decoding');
})();
