import fs from 'node:fs'
const text = fs.readFileSync('./packages/jekko/test/session/daemon-after-checkpoint-hook.test.ts', 'utf8')
const m = text.match(/const zyalWithAfterCheckpointHook = `([\s\S]*?)`/)
const zyal = m?.[1] ?? ''
const tail = zyal.slice(-40)
console.log('tail', JSON.stringify(tail))
for (let i=0; i<tail.length; i++) {
  const c = tail[i]
  process.stdout.write(`${c}:${tail.charCodeAt(i)} `)
}
console.log('\n')
