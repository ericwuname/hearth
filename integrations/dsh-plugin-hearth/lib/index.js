// dsh-plugin-hearth —— 把 Hearth CLI 暴露成 DeepSeek Harness 的 host 工具。
//
// 设计约束（按 DSH 0.2.1-rc 的 ToolDefinition 契约）：
// - 每个工具 = { name, description, parameters, output:{schema,render}, execute }
// - execute 只返回 output.schema 声明的**规范化 JSON 值**（这里统一 {text}）
// - 注册返回精确 disposer；插件卸载时逐个执行（Cordis 可逆效应）
// - 子进程一律 execFile（参数数组、不经 shell），hearth 路径可经 HEARTH_BIN 覆盖
// - 默认不开启会话级审批委托：`hearth chat` 在非交互下会拒绝破坏性操作（fail-closed）

import { execFile } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { dirname, isAbsolute, join } from 'node:path'
import { promisify } from 'node:util'

const execFileAsync = promisify(execFile)

export const name = 'hearth-tools'
export const inject = ['tools']

const MAX_TEXT = 6000
// CLI surface this plugin relies on (`sessions`, `session export`, `chat --budget`).
// Bump when the plugin needs a newer Hearth; the status tool reports incompatibility.
const MIN_HEARTH_VERSION = [0, 2, 27]

function parseVersion(text) {
  const match = String(text ?? '').match(/(\d+)\.(\d+)\.(\d+)/)
  return match ? [Number(match[1]), Number(match[2]), Number(match[3])] : null
}

function versionAtLeast(actual, minimum) {
  if (!actual) return false
  for (let index = 0; index < minimum.length; index += 1) {
    if (actual[index] > minimum[index]) return true
    if (actual[index] < minimum[index]) return false
  }
  return true
}

function hearthBin() {
  const fromEnv = process.env.HEARTH_BIN
  if (fromEnv && fromEnv.trim()) return fromEnv.trim()
  const win = process.platform === 'win32'
  const candidate = join(homedir(), '.cargo', 'bin', win ? 'hearth.exe' : 'hearth')
  return existsSync(candidate) ? candidate : 'hearth'
}

function clip(value, limit = MAX_TEXT) {
  const text = String(value ?? '')
  return text.length > limit ? `${text.slice(0, limit)}\n[truncated ${text.length - limit} chars]` : text
}

function tail(value, limit = 3000) {
  const text = String(value ?? '')
  return text.length > limit ? `[truncated ${text.length - limit} chars]\n${text.slice(-limit)}` : text
}

async function runHearth(args, { cwd, timeout = 120000, signal } = {}) {
  const result = await execFileAsync(hearthBin(), args, {
    cwd,
    timeout,
    signal,
    windowsHide: true,
    maxBuffer: 8 * 1024 * 1024,
    env: process.env,
  })
  return { code: 0, stdout: result.stdout ?? '', stderr: result.stderr ?? '' }
}

async function runHearthSafe(args, options) {
  try {
    return await runHearth(args, options)
  } catch (error) {
    return {
      code: typeof error.code === 'number' ? error.code : 1,
      stdout: error.stdout ?? '',
      stderr: error.stderr ?? String(error.message ?? error),
    }
  }
}

function textOutput() {
  return {
    schema: {
      type: 'object',
      additionalProperties: false,
      properties: { text: { type: 'string' } },
      required: ['text'],
    },
    render: (_args, value) => [{ type: 'text', text: String(value?.text ?? '') }],
  }
}

function fail(prefix, result) {
  const parts = [`${prefix} (exit ${result.code})`]
  if (result.stdout?.trim()) parts.push(`stdout:\n${tail(result.stdout)}`)
  if (result.stderr?.trim()) parts.push(`stderr:\n${tail(result.stderr)}`)
  return parts.join('\n')
}

/**
 * Resolve the directory a Hearth call should run in: explicit argument first,
 * then the agent's session working directory, then the host process cwd.
 */
async function resolveCwd(ctx, exec, explicit) {
  if (explicit) {
    const cwd = String(explicit)
    if (!isAbsolute(cwd)) throw new Error('cwd must be an absolute path')
    if (!existsSync(cwd) || !statSync(cwd).isDirectory()) throw new Error('cwd is not a directory')
    return cwd
  }
  const workingDirectory = typeof ctx?.get === 'function' ? ctx.get('workingDirectory') : undefined
  if (workingDirectory && exec?.agent) {
    try {
      const cwd = await workingDirectory.ensure(exec.agent, exec.signal)
      if (cwd && existsSync(cwd)) return cwd
    } catch {
      // Fall back to the host cwd when the session directory cannot be resolved.
    }
  }
  return process.cwd()
}

/**
 * Resolve the full session id a run printed. Hearth's progress line may show
 * only an 8-char prefix, while export needs the full UUID.
 */
async function resolveSessionId(combined, signal) {
  const full = combined.match(/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/)
  if (full) return full[0]
  const short = combined.match(/session\s+([0-9a-fA-F]{8,})/i)
  if (!short) return null
  const listed = await runHearthSafe(['sessions'], { timeout: 30000, signal })
  if (listed.code !== 0) return null
  const line = listed.stdout
    .split(/\r?\n/)
    .find((entry) => entry.trim().toLowerCase().startsWith(short[1].toLowerCase()))
  const id = line?.match(/^([0-9a-fA-F-]{36})/)
  return id ? id[1] : null
}

/** Export a finished run to a temp file and read back its file changes. */
async function readChanges(sessionId, cwd) {
  const out = join(tmpdir(), `hearth-run-${sessionId}.json`)
  const result = await runHearthSafe(['session', 'export', sessionId, '--out', out], {
    cwd,
    timeout: 60000,
  })
  if (result.code !== 0) return null
  try {
    const document = JSON.parse(readFileSync(out, 'utf8'))
    return Array.isArray(document.changes) ? document.changes : []
  } catch {
    return null
  }
}

/** Latest markdown report Hearth wrote for this session, when one exists. */
function latestReport(cwd, sessionId) {
  const directory = join(cwd, '.hearth', 'reports', sessionId)
  try {
    const files = readdirSync(directory).filter((file) => file.endsWith('.md')).sort()
    return files.length > 0 ? join(directory, files[files.length - 1]) : null
  } catch {
    return null
  }
}

const sessionsTool = {
  name: 'hearth_sessions',
  description:
    'List local Hearth sessions (session id + turn count). Read-only. 列出本机 Hearth 会话，用于挑选要导出的 id。',
  parameters: { type: 'object', properties: {}, additionalProperties: false },
  output: textOutput(),
  async execute(_args, exec) {
    const result = await runHearthSafe(['sessions'], { timeout: 30000, signal: exec?.signal })
    if (result.code !== 0) return { text: fail('hearth sessions failed', result) }
    const body = result.stdout.trim()
    return { text: `hearth sessions:\n${clip(body || '(none)')}` }
  },
}

function exportTool(ctx) {
  return {
    name: 'hearth_export',
    description:
      'Export one Hearth session as a self-contained evidence JSON (diffs + reports + transcript). Read-only on the session. 导出指定会话的证据 JSON，返回落盘路径。',
    parameters: {
      type: 'object',
      properties: {
        id: { type: 'string', description: 'Hearth session UUID (from hearth_sessions)' },
        cwd: { type: 'string', description: 'Absolute working directory for the default export path' },
        out: { type: 'string', description: 'Absolute output JSON path' },
      },
      required: ['id'],
      additionalProperties: false,
    },
    output: textOutput(),
    async execute(args, exec) {
      const id = String(args?.id ?? '').trim()
      if (!/^[0-9a-fA-F-]{8,64}$/.test(id)) throw new Error('invalid Hearth session id')
      const cwd = await resolveCwd(ctx, exec, args?.cwd)
      const out = args?.out ? String(args.out) : join(cwd, '.hearth', 'exports', `${id}.json`)
      if (!isAbsolute(out)) throw new Error('out must be an absolute path')
      mkdirSync(dirname(out), { recursive: true })
      const result = await runHearthSafe(['session', 'export', id, '--out', out], {
        cwd,
        timeout: 60000,
        signal: exec?.signal,
      })
      if (result.code !== 0) return { text: fail('hearth session export failed', result) }
      return { text: `exported → ${out}` }
    },
  }
}

const statusTool = {
  name: 'hearth_status',
  description:
    'Show Hearth runtime status: CLI version, resolved provider/model, and local session count. Read-only. 查看 hearth 版本、provider/model 与会话数量。',
  parameters: { type: 'object', properties: {}, additionalProperties: false },
  output: textOutput(),
  async execute(_args, exec) {
    const version = await runHearthSafe(['--version'], { timeout: 20000, signal: exec?.signal })
    const provider = await runHearthSafe(['config', 'get', 'provider'], {
      timeout: 20000,
      signal: exec?.signal,
    })
    const sessions = await runHearthSafe(['sessions'], { timeout: 30000, signal: exec?.signal })
    const count =
      sessions.code === 0
        ? sessions.stdout.split(/\r?\n/).filter((line) => /\[local\]/.test(line)).length
        : null
    const versionText = version.code === 0 ? version.stdout.trim() : ''
    const compatible = versionAtLeast(parseVersion(versionText), MIN_HEARTH_VERSION)
    const lines = [
      `version: ${versionText || `failed (exit ${version.code})`}`,
      `plugin compatibility: ${compatible ? 'ok' : `requires hearth >= ${MIN_HEARTH_VERSION.join('.')}`}`,
      `provider: ${provider.code === 0 ? provider.stdout.trim() : `failed (exit ${provider.code})`}`,
      `local sessions: ${count === null ? 'unknown' : count}`,
      `binary: ${hearthBin()}`,
    ]
    return { text: lines.join('\n') }
  },
}

function runTool(ctx) {
  return {
    name: 'hearth_run',
    description:
      'Run one Hearth task in a working directory and return its summary. Non-interactive: destructive commands are DENIED unless approve_within_session=true (that delegates session-wide approval — use only when the user explicitly trusts the task). 运行一次 Hearth 任务；默认 fail-closed，破坏性命令会被拒绝。',
    parameters: {
      type: 'object',
      properties: {
        goal: { type: 'string', description: 'Task goal passed to `hearth chat`' },
        budget: { type: 'integer', description: 'Max steps (4-80, default 20)' },
        cwd: {
          type: 'string',
          description: 'Absolute working directory; defaults to the session working directory',
        },
        approve_within_session: {
          type: 'boolean',
          description:
            'Delegate destructive-command approval for this Hearth session (default false). 高风险：开启后命令表级破坏性操作自动放行。',
        },
      },
      required: ['goal'],
      additionalProperties: false,
    },
    output: textOutput(),
    async execute(args, exec) {
      const goal = String(args?.goal ?? '').trim()
      if (!goal) throw new Error('goal is required')
      const rawBudget = Number(args?.budget ?? 20)
      if (!Number.isFinite(rawBudget)) throw new Error('budget must be an integer')
      const budget = Math.min(80, Math.max(4, Math.trunc(rawBudget)))
      const cwd = await resolveCwd(ctx, exec, args?.cwd)
      const argv = ['chat', goal, '--budget', String(budget)]
      if (args?.approve_within_session === true) argv.push('--approve-within', 'session')
      const result = await runHearthSafe(argv, {
        cwd,
        timeout: 600000,
        signal: exec?.signal,
      })
      if (result.code !== 0) return { text: fail('hearth chat failed', result) }
      const combined = `${result.stdout}\n${result.stderr}`
      const session = await resolveSessionId(combined, exec?.signal)
      const head = [
        `ok (budget ${budget}${args?.approve_within_session === true ? ', approval delegated' : ''})`,
        session ? `session: ${session}` : null,
      ]
        .filter(Boolean)
        .join('\n')
      let summary = head
      if (session) {
        const changes = await readChanges(session, cwd)
        if (changes) {
          summary += `\nchanged files: ${changes.length}`
          for (const change of changes.slice(0, 10)) {
            const lines =
              change.lines_before !== undefined && change.lines_after !== undefined
                ? ` (${change.lines_before}→${change.lines_after} lines)`
                : ''
            summary += `\n- ${change.path}${change.created ? ' (created)' : ''}${lines}`
          }
        }
        const report = latestReport(cwd, session)
        if (report) summary += `\nreport: ${report}`
      }
      return { text: `${summary}\n${clip(combined.trim(), 5000)}` }
    },
  }
}

export function apply(ctx) {
  const TOOLS = [sessionsTool, exportTool(ctx), statusTool, runTool(ctx)]
  const disposers = TOOLS.map((tool) => ctx.tools.register(tool))
  return () => {
    for (const dispose of disposers.reverse()) {
      try {
        dispose()
      } catch {
        // 卸载异常不阻断其余工具的清理；由 Cordis 记录
      }
    }
  }
}
