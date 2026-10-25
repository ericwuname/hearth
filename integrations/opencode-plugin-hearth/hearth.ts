/**
 * Hearth tools for OpenCode Desktop v2 (Promise plugin API).
 *
 * Drop this file into `~/.config/opencode/plugins/hearth.ts` (or
 * `.opencode/plugins/hearth.ts`). The v2 loader resolves `@opencode/plugin`
 * from the host, so no local node_modules are required.
 */
import { execFile } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { dirname, isAbsolute, join } from 'node:path'
import { promisify } from 'node:util'
import { Plugin } from '@opencode/plugin'

const execFileAsync = promisify(execFile)
const MAX_TEXT = 6000
const MIN_HEARTH_VERSION = [0, 2, 27]

function hearthBin(): string {
  const fromEnv = process.env.HEARTH_BIN
  if (fromEnv && fromEnv.trim()) return fromEnv.trim()
  const win = process.platform === 'win32'
  const candidate = join(homedir(), '.cargo', 'bin', win ? 'hearth.exe' : 'hearth')
  return existsSync(candidate) ? candidate : 'hearth'
}

function clip(value: unknown, limit = MAX_TEXT): string {
  const text = String(value ?? '')
  return text.length > limit ? `${text.slice(0, limit)}\n[truncated ${text.length - limit} chars]` : text
}

function tail(value: unknown, limit = 3000): string {
  const text = String(value ?? '')
  return text.length > limit ? `[truncated ${text.length - limit} chars]\n${text.slice(-limit)}` : text
}

async function runHearth(
  args: string[],
  options: { cwd?: string; timeout?: number; signal?: AbortSignal } = {},
): Promise<{ code: number; stdout: string; stderr: string }> {
  try {
    const result = await execFileAsync(hearthBin(), args, {
      cwd: options.cwd,
      timeout: options.timeout ?? 120000,
      signal: options.signal,
      windowsHide: true,
      maxBuffer: 8 * 1024 * 1024,
      env: process.env,
    })
    return { code: 0, stdout: result.stdout ?? '', stderr: result.stderr ?? '' }
  } catch (error) {
    const failure = error as { code?: number; stdout?: string; stderr?: string; message?: string }
    return {
      code: typeof failure.code === 'number' ? failure.code : 1,
      stdout: failure.stdout ?? '',
      stderr: failure.stderr ?? String(failure.message ?? failure),
    }
  }
}

function fail(prefix: string, result: { code: number; stdout: string; stderr: string }): string {
  const parts = [`${prefix} (exit ${result.code})`]
  if (result.stdout.trim()) parts.push(`stdout:\n${tail(result.stdout)}`)
  if (result.stderr.trim()) parts.push(`stderr:\n${tail(result.stderr)}`)
  return parts.join('\n')
}

function textContent(text: string) {
  return [{ type: 'text' as const, text }]
}

interface Change {
  path?: string
  created?: boolean
  state?: string
  lines_before?: number
  lines_after?: number
}

/**
 * Resolve the full session id a run printed. Hearth's progress line may show
 * only an 8-char prefix, while export needs the full UUID.
 */
async function resolveSessionId(combined: string, signal?: AbortSignal): Promise<string | null> {
  const full = combined.match(/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/)
  if (full) return full[0]
  const short = combined.match(/session\s+([0-9a-fA-F]{8,})/i)
  if (!short) return null
  const listed = await runHearth(['sessions'], { timeout: 30000, signal })
  if (listed.code !== 0) return null
  const line = listed.stdout
    .split(/\r?\n/)
    .find((entry) => entry.trim().toLowerCase().startsWith(short[1].toLowerCase()))
  const id = line?.match(/^([0-9a-fA-F-]{36})/)
  return id ? id[1] : null
}

/** Read the exported evidence after a run and return its file changes. */
async function readChanges(sessionId: string, cwd: string | undefined): Promise<Change[] | null> {
  const out = join(tmpdir(), `hearth-run-${sessionId}.json`)
  const result = await runHearth(['session', 'export', sessionId, '--out', out], { cwd, timeout: 60000 })
  if (result.code !== 0) return null
  try {
    const document = JSON.parse(readFileSync(out, 'utf8')) as { changes?: Change[] }
    return Array.isArray(document.changes) ? document.changes : []
  } catch {
    return null
  }
}

/** Latest markdown report Hearth wrote for this session, when one exists. */
function latestReport(cwd: string, sessionId: string): string | null {
  const directory = join(cwd, '.hearth', 'reports', sessionId)
  try {
    const files = readdirSync(directory).filter((file) => file.endsWith('.md')).sort()
    return files.length > 0 ? join(directory, files[files.length - 1]) : null
  } catch {
    return null
  }
}

const emptySchema = { type: 'object', properties: {}, additionalProperties: false }

export default Plugin.define({
  id: 'hearth',
  async setup(ctx) {
    // Default Hearth tasks to the OpenCode project directory, not the app cwd.
    const workspace = ctx.location?.directory
    await ctx.tool.transform((editor) => {
      editor.add({
        name: 'hearth_sessions',
        description: 'List local Hearth sessions (id + turn count). Read-only.',
        input: emptySchema,
        execute: async (_input, tool) => {
          const result = await runHearth(['sessions'], { timeout: 30000, signal: tool.signal })
          if (result.code !== 0) return { content: textContent(fail('hearth sessions failed', result)) }
          return { content: textContent(`hearth sessions:\n${clip(result.stdout.trim() || '(none)')}`) }
        },
      })

      editor.add({
        name: 'hearth_export',
        description: 'Export one Hearth session as a self-contained evidence JSON. Read-only.',
        input: {
          type: 'object',
          properties: {
            id: { type: 'string', description: 'Hearth session UUID' },
            cwd: { type: 'string', description: 'Absolute working directory for the default export path' },
            out: { type: 'string', description: 'Absolute output JSON path' },
          },
          required: ['id'],
          additionalProperties: false,
        },
        execute: async (input, tool) => {
          const args = input as { id?: string; cwd?: string; out?: string }
          const id = String(args.id ?? '').trim()
          if (!/^[0-9a-fA-F-]{8,64}$/.test(id)) throw new Error('invalid Hearth session id')
          const cwd = args.cwd
            ? String(args.cwd)
            : workspace && existsSync(workspace) && statSync(workspace).isDirectory()
              ? workspace
              : homedir()
          if (!isAbsolute(cwd)) throw new Error('cwd must be an absolute path')
          const out = args.out
            ? String(args.out)
            : join(cwd, '.hearth', 'exports', `${id}.json`)
          if (!isAbsolute(out)) throw new Error('out must be an absolute path')
          mkdirSync(dirname(out), { recursive: true })
          const result = await runHearth(['session', 'export', id, '--out', out], {
            cwd,
            timeout: 60000,
            signal: tool.signal,
          })
          if (result.code !== 0) return { content: textContent(fail('hearth session export failed', result)) }
          return { content: textContent(`exported → ${out}`) }
        },
      })

      editor.add({
        name: 'hearth_status',
        description: 'Show Hearth CLI version, provider, local session count and plugin compatibility.',
        input: emptySchema,
        execute: async (_input, tool) => {
          const version = await runHearth(['--version'], { timeout: 20000, signal: tool.signal })
          const provider = await runHearth(['config', 'get', 'provider'], { timeout: 20000, signal: tool.signal })
          const sessions = await runHearth(['sessions'], { timeout: 30000, signal: tool.signal })
          const versionText = version.code === 0 ? version.stdout.trim() : ''
          const match = versionText.match(/(\d+)\.(\d+)\.(\d+)/)
          const actual = match ? [Number(match[1]), Number(match[2]), Number(match[3])] : null
          const compatible = actual !== null && actual[0] >= MIN_HEARTH_VERSION[0]
            && (actual[0] > MIN_HEARTH_VERSION[0] || actual[1] > MIN_HEARTH_VERSION[1]
              || (actual[1] === MIN_HEARTH_VERSION[1] && actual[2] >= MIN_HEARTH_VERSION[2]))
          const count = sessions.code === 0
            ? sessions.stdout.split(/\r?\n/).filter((line) => line.includes('[local]')).length
            : null
          return {
            content: textContent([
              `version: ${versionText || `failed (exit ${version.code})`}`,
              `plugin compatibility: ${compatible ? 'ok' : `requires hearth >= ${MIN_HEARTH_VERSION.join('.')}`}`,
              `provider: ${provider.code === 0 ? provider.stdout.trim() : `failed (exit ${provider.code})`}`,
              `local sessions: ${count ?? 'unknown'}`,
              `binary: ${hearthBin()}`,
            ].join('\n')),
          }
        },
      })

      editor.add({
        name: 'hearth_run',
        description:
          'Run one Hearth task. Non-interactive: destructive commands are DENIED unless approve_within_session=true.',
        input: {
          type: 'object',
          properties: {
            goal: { type: 'string', description: 'Task goal passed to `hearth chat`' },
            budget: { type: 'integer', description: 'Max steps (4-80, default 20)' },
            cwd: { type: 'string', description: 'Absolute working directory' },
            approve_within_session: { type: 'boolean', description: 'Delegate destructive-command approval (high risk)' },
          },
          required: ['goal'],
          additionalProperties: false,
        },
        execute: async (input, tool) => {
          const args = input as { goal?: string; budget?: number; cwd?: string; approve_within_session?: boolean }
          const goal = String(args.goal ?? '').trim()
          if (!goal) throw new Error('goal is required')
          const rawBudget = Number(args.budget ?? 20)
          if (!Number.isFinite(rawBudget)) throw new Error('budget must be an integer')
          const budget = Math.min(80, Math.max(4, Math.trunc(rawBudget)))
          let cwd: string | undefined
          if (args.cwd) {
            cwd = String(args.cwd)
            if (!isAbsolute(cwd)) throw new Error('cwd must be an absolute path')
            if (!existsSync(cwd) || !statSync(cwd).isDirectory()) throw new Error('cwd is not a directory')
          } else if (workspace && existsSync(workspace) && statSync(workspace).isDirectory()) {
            cwd = workspace
          }
          const argv = ['chat', goal, '--budget', String(budget)]
          if (args.approve_within_session === true) argv.push('--approve-within', 'session')
          const result = await runHearth(argv, { cwd, timeout: 600000, signal: tool.signal })
          if (result.code !== 0) return { content: textContent(fail('hearth chat failed', result)) }
          const combined = `${result.stdout}\n${result.stderr}`
          const session = await resolveSessionId(combined, tool.signal)
          const head = `ok (budget ${budget}${args.approve_within_session === true ? ', approval delegated' : ''})`
          let summary = ''
          if (session) {
            summary += `\nsession: ${session}`
            const changes = await readChanges(session, cwd)
            if (changes) {
              summary += `\nchanged files: ${changes.length}`
              for (const change of changes.slice(0, 10)) {
                const lines = change.lines_before !== undefined && change.lines_after !== undefined
                  ? ` (${change.lines_before}→${change.lines_after} lines)`
                  : ''
                summary += `\n- ${change.path}${change.created ? ' (created)' : ''}${lines}`
              }
            }
            if (cwd) {
              const report = latestReport(cwd, session)
              if (report) summary += `\nreport: ${report}`
            }
          }
          return { content: textContent(`${head}${summary}\n${clip(combined.trim(), 5000)}`) }
        },
      })
    })
  },
})
