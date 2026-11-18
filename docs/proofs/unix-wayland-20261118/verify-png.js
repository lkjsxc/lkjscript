'use strict';
// Independent exact pixel verifier; built-in Node zlib only, no renderer import.
const fs = require('fs'), zlib = require('zlib'), crypto = require('crypto');
const bytes = fs.readFileSync(process.argv[2]);
if (!bytes.subarray(0, 8).equals(Buffer.from([137,80,78,71,13,10,26,10]))) throw Error('PNG signature');
let width, height, channels, chunks = [], at = 8;
while (at < bytes.length) {
  const length = bytes.readUInt32BE(at), type = bytes.toString('ascii', at + 4, at + 8), data = bytes.subarray(at + 8, at + 8 + length);
  if (type === 'IHDR') {
    width = data.readUInt32BE(0); height = data.readUInt32BE(4); channels = data[9] === 2 ? 3 : data[9] === 6 ? 4 : 0;
    if (data[8] !== 8 || !channels || data[10] || data[11] || data[12]) throw Error('unsupported screenshot PNG');
  }
  if (type === 'IDAT') chunks.push(data);
  at += length + 12;
}
if (width !== 320 || height !== 240) throw Error(`expected 320x240 compositor output, got ${width}x${height}`);
const raw = zlib.inflateSync(Buffer.concat(chunks)), stride = width * channels, decoded = Buffer.alloc(stride * height);
if (raw.length !== (stride + 1) * height) throw Error('PNG decoded length');
function paeth(a,b,c) { const p=a+b-c, pa=Math.abs(p-a), pb=Math.abs(p-b), pc=Math.abs(p-c); return pa<=pb&&pa<=pc?a:pb<=pc?b:c; }
for (let y=0;y<height;y++) {
  const filter=raw[y*(stride+1)]; if(filter>4) throw Error('PNG filter');
  for(let x=0;x<stride;x++) {
    const a=x>=channels?decoded[y*stride+x-channels]:0,b=y?decoded[(y-1)*stride+x]:0,c=y&&x>=channels?decoded[(y-1)*stride+x-channels]:0;
    const predictor=filter===0?0:filter===1?a:filter===2?b:filter===3?Math.floor((a+b)/2):paeth(a,b,c);
    decoded[y*stride+x]=(raw[y*(stride+1)+1+x]+predictor)&255;
  }
}
let matched=0;
for(let y=0;y<height;y++) for(let x=0;x<width;x++) {
  const pixel=decoded.subarray(y*stride+x*channels,y*stride+(x+1)*channels),expected=[30+Math.floor(y/2),20+Math.floor(x/2),40+Math.floor(y/2)];
  if(expected.some((v,i)=>pixel[i]!==v)||(channels===4&&pixel[3]!==255)) throw Error(`compositor pixel mismatch ${x},${y}: got ${[...pixel]} expected ${expected}`);
  matched++;
}
process.stdout.write(JSON.stringify({width,height,matched_pixels:matched,matched_rgb_bytes:matched*3,screenshot_sha256:crypto.createHash('sha256').update(bytes).digest('hex')},null,2)+'\n');
