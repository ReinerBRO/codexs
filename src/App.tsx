import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { startTransition, useEffect, useId, useState } from 'react'
import './App.css'
import type {
  Account,
  GenerationProgressEvent,
  GenerationResult,
  ImportResult,
  NoticeState,
} from './types'

const DEFAULT_COUNT = '10'
const GENERATION_PROGRESS_EVENT = 'generation_progress'

const initialProgress: GenerationProgressEvent = {
  current: 0,
  total: 0,
  email: '',
}

const initialNotice: NoticeState = {
  tone: 'neutral',
  text: '准备就绪，等待开始生成。',
}

function isTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function getErrorMessage(error: unknown) {
  if (typeof error === 'string') {
    return error
  }

  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    return error.message
  }

  return '发生未知错误'
}

function formatTimestamp(value: string) {
  if (!value) {
    return '时间未知'
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(date)
}

function App() {
  const countId = useId()
  const [accounts, setAccounts] = useState<Account[]>([])
  const [selectedEmails, setSelectedEmails] = useState<string[]>([])
  const [countInput, setCountInput] = useState(DEFAULT_COUNT)
  const [progress, setProgress] = useState<GenerationProgressEvent>(initialProgress)
  const [notice, setNotice] = useState<NoticeState>(initialNotice)
  const [recentErrors, setRecentErrors] = useState<string[]>([])
  const [isGenerating, setIsGenerating] = useState(false)
  const [isImporting, setIsImporting] = useState(false)
  const [isHydrating, setIsHydrating] = useState(true)
  const [runtimeReady, setRuntimeReady] = useState(false)

  const parsedCount = Number.parseInt(countInput, 10)
  const requestedCount = Number.isFinite(parsedCount) ? parsedCount : 0
  const totalProgress = progress.total > 0 ? progress.total : requestedCount
  const progressPercent =
    totalProgress > 0
      ? Math.min(100, Math.round((progress.current / totalProgress) * 100))
      : 0
  const selectedCount = selectedEmails.length
  const importedCount = accounts.filter((account) => account.imported).length
  const pendingCount = accounts.length - importedCount
  const canGenerate =
    runtimeReady && !isGenerating && !isImporting && Number.isInteger(requestedCount) && requestedCount > 0
  const canImport = runtimeReady && !isGenerating && !isImporting && selectedCount > 0

  useEffect(() => {
    if (!isTauriRuntime()) {
      startTransition(() => {
        setRuntimeReady(false)
        setIsHydrating(false)
        setNotice({
          tone: 'neutral',
          text: '当前是浏览器预览模式，Tauri commands 仅在桌面端可用。',
        })
      })
      return
    }

    let disposed = false
    let unlisten: (() => void) | undefined

    void (async () => {
      try {
        setRuntimeReady(true)
        const nextAccounts = await invoke<Account[]>('get_accounts')
        const availableEmails = new Set(nextAccounts.map((account) => account.email))

        startTransition(() => {
          setAccounts(nextAccounts)
          setSelectedEmails((current) =>
            current.filter((email) => availableEmails.has(email)),
          )
        })

        unlisten = await listen<GenerationProgressEvent>(
          GENERATION_PROGRESS_EVENT,
          (event) => {
            if (disposed) {
              return
            }

            startTransition(() => {
              setProgress(event.payload)
            })
          },
        )
      } catch (error) {
        if (disposed) {
          return
        }

        setNotice({
          tone: 'error',
          text: `初始化失败：${getErrorMessage(error)}`,
        })
      } finally {
        if (!disposed) {
          setIsHydrating(false)
        }
      }
    })()

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [])

  const toggleSelection = (email: string) => {
    setSelectedEmails((current) =>
      current.includes(email)
        ? current.filter((item) => item !== email)
        : [...current, email],
    )
  }

  const handleGenerate = async () => {
    if (!canGenerate) {
      return
    }

    setIsGenerating(true)
    setRecentErrors([])
    setProgress({
      current: 0,
      total: requestedCount,
      email: '',
    })
    setNotice({
      tone: 'neutral',
      text: `正在生成 ${requestedCount} 个账号...`,
    })

    try {
      const result = await invoke<GenerationResult>('start_generation', {
        count: requestedCount,
      })
      const nextAccounts = await invoke<Account[]>('get_accounts')
      const availableEmails = new Set(nextAccounts.map((account) => account.email))

      startTransition(() => {
        setAccounts(nextAccounts)
        setSelectedEmails((current) =>
          current.filter((email) => availableEmails.has(email)),
        )
      })

      setRecentErrors(result.errors)
      setProgress((current) => ({
        current: result.requested,
        total: result.requested,
        email: result.accounts.at(-1)?.email ?? current.email,
      }))
      setNotice({
        tone: result.failed > 0 ? 'error' : 'success',
        text:
          result.failed > 0
            ? `生成完成，成功 ${result.succeeded} 个，失败 ${result.failed} 个。`
            : `生成完成，已新增 ${result.succeeded} 个账号。`,
      })
    } catch (error) {
      const message = getErrorMessage(error)
      setRecentErrors([message])
      setNotice({
        tone: 'error',
        text: `生成失败：${message}`,
      })
    } finally {
      setIsGenerating(false)
    }
  }

  const handleImport = async () => {
    if (!canImport) {
      return
    }

    const emails = [...selectedEmails]
    setIsImporting(true)
    setNotice({
      tone: 'neutral',
      text: `正在导入 ${emails.length} 个选中账号...`,
    })

    try {
      const result = await invoke<ImportResult>('import_accounts', { emails })
      const nextAccounts = await invoke<Account[]>('get_accounts')

      startTransition(() => {
        setAccounts(nextAccounts)
      })
      setSelectedEmails([])
      setNotice({
        tone: result.failed > 0 ? 'error' : 'success',
        text:
          result.failed > 0
            ? `导入完成，已处理 ${result.imported} 个，失败 ${result.failed} 个。`
            : `导入完成，已处理 ${result.imported} 个账号。`,
      })
    } catch (error) {
      setNotice({
        tone: 'error',
        text: `导入失败：${getErrorMessage(error)}`,
      })
    } finally {
      setIsImporting(false)
    }
  }

  const currentEmail =
    progress.email ||
    (progress.current > 0 ? '本轮未产出邮箱' : '等待生成任务')

  return (
    <main className="app-shell">
      <section className="app-frame">
        <header className="frame-header">
          <div className="frame-title">
            <p className="frame-kicker">Minimal Tech Console</p>
            <h1>codexs</h1>
          </div>

          <div className={`runtime-indicator ${runtimeReady ? 'is-live' : ''}`}>
            <span className="runtime-dot" />
            {runtimeReady ? 'Tauri Connected' : 'Browser Preview'}
          </div>
        </header>

        <div className={`status-banner tone-${notice.tone}`}>
          <span className="status-marker" />
          <span>{notice.text}</span>
        </div>

        <section className="overview-grid">
          <article className="panel">
            <div className="panel-heading">
              <p className="panel-tag">生成配置</p>
              <h2>批量生成账号</h2>
            </div>

            <label className="field-card" htmlFor={countId}>
              <span className="field-label">数量</span>
              <div className="field-input">
                <input
                  id={countId}
                  type="text"
                  inputMode="numeric"
                  value={countInput}
                  onChange={(event) => {
                    const nextValue = event.target.value.replace(/\D+/g, '')
                    setCountInput(nextValue)
                  }}
                  placeholder={DEFAULT_COUNT}
                  disabled={isGenerating || isImporting}
                />
                <span className="field-suffix">个</span>
              </div>
            </label>

            <button
              className="action-button primary"
              type="button"
              onClick={handleGenerate}
              disabled={!canGenerate}
            >
              {isGenerating ? '生成中...' : '开始生成'}
            </button>

            <div className="meta-row">
              <span>{isHydrating ? '读取账号列表中...' : `${accounts.length} 个账号已载入`}</span>
              <span>{requestedCount > 0 ? `本次计划 ${requestedCount} 个` : '请输入有效数量'}</span>
            </div>
          </article>

          <article className="panel">
            <div className="panel-heading">
              <p className="panel-tag">进度显示</p>
              <h2>生成进度</h2>
            </div>

            <div className="progress-summary">
              <div>
                <strong>{`${progress.current}/${totalProgress || 0}`}</strong>
                <span>{progressPercent}%</span>
              </div>
              <span className={`progress-state ${isGenerating ? 'is-active' : ''}`}>
                {isGenerating
                  ? '生成中'
                  : totalProgress > 0 && progress.current >= totalProgress
                    ? '已完成'
                    : '待命'}
              </span>
            </div>

            <div className="progress-track" aria-hidden="true">
              <div
                className="progress-fill"
                style={{ width: `${progressPercent}%` }}
              />
            </div>

            <div className="progress-card">
              <span className="progress-label">当前</span>
              <strong className="progress-email">{currentEmail}</strong>
            </div>

            {recentErrors.length > 0 ? (
              <div className="diagnostics">
                <p>最近异常</p>
                <ul>
                  {recentErrors.slice(0, 3).map((error) => (
                    <li key={error}>{error}</li>
                  ))}
                </ul>
              </div>
            ) : null}
          </article>
        </section>

        <section className="panel accounts-panel">
          <div className="section-row">
            <div>
              <p className="panel-tag">账号列表</p>
              <h2>{`账号列表 (${accounts.length})`}</h2>
            </div>

            <button
              className="action-button secondary"
              type="button"
              onClick={handleImport}
              disabled={!canImport}
            >
              {isImporting ? '导入中...' : `导入选中账号${selectedCount > 0 ? ` (${selectedCount})` : ''}`}
            </button>
          </div>

          <div className="summary-chips" aria-label="账号统计">
            <span>{`已导入 ${importedCount}`}</span>
            <span>{`未导入 ${pendingCount}`}</span>
            <span>{`已选中 ${selectedCount}`}</span>
          </div>

          {accounts.length > 0 ? (
            <div className="account-list" role="list">
              {accounts.map((account) => {
                const checked = selectedEmails.includes(account.email)

                return (
                  <label
                    className={`account-row ${checked ? 'is-selected' : ''}`}
                    key={account.email}
                  >
                    <span className="account-check">
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={() => toggleSelection(account.email)}
                        disabled={isGenerating || isImporting}
                      />
                      <span className="checkbox-mark" />
                    </span>

                    <span className="account-copy">
                      <span className="account-email">{account.email}</span>
                      <span className="account-time">
                        {formatTimestamp(account.created_at)}
                      </span>
                    </span>

                    <span
                      className={`account-status ${
                        account.imported ? 'is-imported' : 'is-pending'
                      }`}
                    >
                      <span className="status-dot" />
                      {account.imported ? '已导入' : '未导入'}
                    </span>
                  </label>
                )
              })}
            </div>
          ) : (
            <div className="empty-state">
              <strong>还没有账号</strong>
              <p>先在上方设置数量并开始生成，列表会自动刷新。</p>
            </div>
          )}
        </section>
      </section>
    </main>
  )
}

export default App
