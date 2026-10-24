// CLI contract gate for the DSH bridge.
// Run this before releasing Hearth: the plugin parses these exact command outputs.
import { execFile } from 'node:child_process'
import { mkdtemp, readFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { join } from 'node:path'
import { promisify } from 'node:util'

const execFileAsync = promisify(execFile)
const win = process.platform === 'win32'
const bin = process.env.HEARTH_BIN || (existsSync(join(homedir(), '.cargo', 'bin', win ? 'hearth.exe' : 'hearth'))
  ? join(homedir(), '.cargo', 'bin', win ? 'hearth.exe' : 'hearth')
  : 'hearth')

async function run(args) {
  const result = await execFileAsync(bin, args, {
    windowsHide: true,
    timeout: 60000,
    maxBuffer: 8 * 1024 * 1024,
    env: process.env,
  })
  return { stdout: result.stdout ?? '', stderr: result.stderr ?? '' }
}

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

const version = (await run(['--version'])).stdout.trim()
assert(/^hearth \d+\.\d+\.\d+/.test(version), `--version format changed: ${JSON.stringify(version)}`)

const provider = (await run(['config', 'get', 'provider'])).stdout.trim()
assert(provider.length > 0, 'config get provider returned empty output')

const sessions = (await run(['sessions'])).stdout
const lines = sessions.split(/\r?\n/).filter((line) => line.trim().length > 0)
assert(lines.length > 0, 'sessions returned no lines')
const rows = lines.filter((line) => /[0-9a-fA-F-]{36}\s+\[local\]\s+turns=\d+/u.test(line))
assert(rows.length > 0, `sessions line format changed: ${JSON.stringify(lines[0])}`)
const sessionId = rows[0].match(/^([0-9a-fA-F-]{36})/u)[1]

const dir = await mkdtemp(join(tmpdir(), 'hearth-plugin-contract-'))
const out = join(dir, 'export.json')
const exported = (await run(['session', 'export', sessionId, '--out', out])).stdout
assert(exported.includes('→'), `session export success line changed: ${JSON.stringify(exported.trim())}`)
const document = JSON.parse(await readFile(out, 'utf8'))
assert(document.session_id === sessionId, 'export session_id mismatch')
assert(typeof document.schema_version === 'number', 'export schema_version missing or changed type')
assert(typeof document.exported_at === 'string', 'export exported_at missing')

console.log('CONTRACT OK')
console.log(`binary: ${bin}`)
console.log(`version: ${version}`)
console.log(`sessions: ${rows.length} parseable`)
console.log(`export: ${out}`)
