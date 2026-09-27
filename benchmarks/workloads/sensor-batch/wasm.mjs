import { readFileSync } from 'node:fs'
import { createPolyXml } from '../../../crates/polyxml-wasm/pkg/index.node.js'

const source = readFileSync(new URL('../../../Cargo.toml', import.meta.url), 'utf8')
const sourceVersion = source.match(/\[workspace\.package\]\s*version = "([^"]+)"/)?.[1]
const packageVersion = JSON.parse(readFileSync(new URL('../../../crates/polyxml-wasm/pkg/package.json', import.meta.url), 'utf8')).version
if (!sourceVersion || packageVersion !== sourceVersion) {
  throw new Error(`Wasm package ${packageVersion} differs from local source ${sourceVersion}; rebuild with ./crates/polyxml-wasm/build.sh`)
}
const wasm = await createPolyXml()
console.log(`node_version=${process.version}`)
for (const [count, iterations] of [[1, 10000], [1000, 100]]) {
  const xml = readFileSync(new URL(`./sensor-${count}.xml`, import.meta.url), 'utf8')
  const first = wasm.xmlToJson(xml).Batch.Sensor
  const sensors = Array.isArray(first) ? first : [first]
  if (sensors.length !== count || sensors.at(-1).Id !== `sensor-${count - 1}` || sensors.at(-1).Value !== count - 1) {
    throw new Error('Wasm decoded unexpected sensor values')
  }
  for (let i = 0; i < 100; i++) wasm.xmlToJson(xml)
  for (let repeat = 0; repeat < 5; repeat++) {
    const start = process.hrtime.bigint()
    let result
    for (let i = 0; i < iterations; i++) result = wasm.xmlToJson(xml)
    const ns = Number(process.hrtime.bigint() - start) / iterations
    console.log(`wasm,size=${count},repeat=${repeat},ns/op=${ns.toFixed(1)},xml_bytes=${Buffer.byteLength(xml)},last_id=${(Array.isArray(result.Batch.Sensor) ? result.Batch.Sensor.at(-1) : result.Batch.Sensor).Id}`)
  }
}
