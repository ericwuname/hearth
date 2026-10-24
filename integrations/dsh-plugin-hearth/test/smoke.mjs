// 独立冒烟：不需要启动 DSH，用假 ctx.tools 驱动插件定义。
// 覆盖：插件导出形状、四个工具的注册契约、read-only 工具真实调用、disposer。
import { apply, inject, name } from '../lib/index.js'

const defs = []
const disposed = []
const ctx = {
  tools: {
    register(def) {
      defs.push(def)
      return () => disposed.push(def.name)
    },
  },
}

const dispose = apply(ctx)

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

assert(name === 'hearth-tools', 'plugin name')
assert(Array.isArray(inject) && inject.includes('tools'), 'plugin inject tools')
assert(defs.length === 4, `expected 4 tools, got ${defs.length}`)

for (const def of defs) {
  assert(typeof def.name === 'string' && def.name.startsWith('hearth_'), `tool name: ${def.name}`)
  assert(typeof def.description === 'string' && def.description.length > 10, `description: ${def.name}`)
  assert(def.parameters && def.parameters.type === 'object', `parameters: ${def.name}`)
  assert(def.output && def.output.schema && typeof def.output.render === 'function', `output: ${def.name}`)
  assert(typeof def.execute === 'function', `execute: ${def.name}`)
}

const byName = Object.fromEntries(defs.map((def) => [def.name, def]))

const sessions = await byName.hearth_sessions.execute({}, {})
assert(typeof sessions.text === 'string', 'hearth_sessions returns text')
assert(sessions.text.includes('hearth sessions:'), 'hearth_sessions header')

const status = await byName.hearth_status.execute({}, {})
assert(status.text.includes('version: hearth'), 'hearth_status version')
assert(status.text.includes('provider:'), 'hearth_status provider')

let rejected = false
try {
  await byName.hearth_run.execute({ goal: 'x', cwd: 'relative/path' }, {})
} catch (error) {
  rejected = /absolute/.test(String(error.message))
}
assert(rejected, 'hearth_run rejects relative cwd')

dispose()
assert(disposed.length === 4, `disposer should unregister 4 tools, got ${disposed.length}`)

console.log('SMOKE OK')
console.log(sessions.text.split('\n').slice(0, 4).join('\n'))
console.log(status.text)
