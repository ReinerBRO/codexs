import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { startTransition, useEffect, useId, useState } from 'react'
import './App.css'
import logo from './assets/logo.png'
import { AccountManagement } from './components/AccountManagement'
import {
  formatGenerationFailure,
  formatGenerationPartialSuccess,
  formatGenerationStopped,
  formatGenerationSuccess,
  formatImportFailure,
  formatImportInProgress,
  formatImportPartialSuccess,
  formatImportSuccess,
  formatInitError,
  getBrowserLanguage,
  getTranslations,
  type Language,
} from './i18n'
import type {
  Account,
  GenerationProgressEvent,
  GenerationResult,
  ImportResult,
  NoticeState,
  ProgressLog,
} from './types'

const DEFAULT_COUNT = '10'
const GENERATION_PROGRESS_EVENT = 'generation_progress'

const initialProgress: GenerationProgressEvent = {
  current: 0,
  total: 0,
  email: '',
  account: null,
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

function sortAccounts(items: Account[]) {
  return [...items].sort((left, right) => {
    if (left.created_at !== right.created_at) {
      return right.created_at.localeCompare(left.created_at)
    }

    return left.email.localeCompare(right.email)
  })
}

function upsertAccount(items: Account[], nextAccount: Account) {
  const nextItems = items.filter((account) => account.email !== nextAccount.email)
  nextItems.push(nextAccount)
  return sortAccounts(nextItems)
}

function App() {
  const countId = useId()
  const [lang, setLang] = useState<Language>(getBrowserLanguage())
  const t = getTranslations(lang)
  const [accounts, setAccounts] = useState<Account[]>([])
  const [selectedEmails, setSelectedEmails] = useState<string[]>([])
  const [countInput, setCountInput] = useState(DEFAULT_COUNT)
  const [progress, setProgress] = useState<GenerationProgressEvent>(initialProgress)
  const [notice, setNotice] = useState<NoticeState>({ tone: 'neutral', text: t.generation.ready })
  const [progressLogs, setProgressLogs] = useState<ProgressLog[]>([])
  const [recentErrors, setRecentErrors] = useState<string[]>([])
  const [isGenerating, setIsGenerating] = useState(false)
  const [isStopping, setIsStopping] = useState(false)
  const [isImporting, setIsImporting] = useState(false)
  const [isHydrating, setIsHydrating] = useState(true)
  const [runtimeReady, setRuntimeReady] = useState(false)
  const [generationStopped, setGenerationStopped] = useState(false)

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
  const canStop = runtimeReady && isGenerating && !isStopping
  const canImport = runtimeReady && !isGenerating && !isImporting && selectedCount > 0

  useEffect(() => {
    if (!isTauriRuntime()) {
      startTransition(() => {
        setRuntimeReady(false)
        setIsHydrating(false)
        setNotice({
          tone: 'neutral',
          text: t.app.browserPreviewNotice,
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

            const payload = event.payload
            startTransition(() => {
              setProgress(payload)
              if (payload.account) {
                setAccounts((current) => upsertAccount(current, payload.account!))
              }
              if (payload.email) {
                setProgressLogs((logs) => [
                  ...logs,
                  {
                    type: 'success',
                    message: `${payload.current}/${payload.total} - ${payload.email} ${t.status.success}`,
                    timestamp: new Date().toISOString(),
                  },
                ])
              }
            })
          },
        )
      } catch (error) {
        if (disposed) {
          return
        }

        setNotice({
          tone: 'error',
          text: formatInitError(lang, getErrorMessage(error)),
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
  }, [t])

  const toggleSelection = (email: string) => {
    setSelectedEmails((current) =>
      current.includes(email)
        ? current.filter((item) => item !== email)
        : [...current, email],
    )
  }

  const selectAllPending = () => {
    const pendingEmails = accounts
      .filter((account) => !account.imported)
      .map((account) => account.email)
    setSelectedEmails(pendingEmails)
  }

  const clearSelection = () => {
    setSelectedEmails([])
  }

  const handleGenerate = async () => {
    if (!canGenerate) {
      return
    }

    setIsGenerating(true)
    setIsStopping(false)
    setGenerationStopped(false)
    setRecentErrors([])
    setProgressLogs([])
    setProgress({
      current: 0,
      total: requestedCount,
      email: '',
      account: null,
    })
    setNotice({
      tone: 'neutral',
      text: `${t.status.generating} ${requestedCount} ${t.generation.countUnit}...`,
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
      setGenerationStopped(result.stopped)
      setProgress((current) => (({
        current: result.stopped ? result.succeeded + result.failed : result.requested,
        total: result.requested,
        email: result.accounts.at(-1)?.email ?? current.email,
        account: current.account,
      })))
      setNotice({
        tone: result.stopped ? (result.failed > 0 ? 'error' : 'neutral') : result.failed > 0 ? 'error' : 'success',
        text:
          result.stopped
            ? formatGenerationStopped(lang, result.succeeded, result.failed)
            : result.failed > 0
              ? formatGenerationPartialSuccess(lang, result.succeeded, result.failed)
              : formatGenerationSuccess(lang, result.succeeded),
      })
    } catch (error) {
      const message = getErrorMessage(error)
      setRecentErrors([message])
      setGenerationStopped(false)
      setNotice({
        tone: 'error',
        text: formatGenerationFailure(lang, message),
      })
    } finally {
      setIsStopping(false)
      setIsGenerating(false)
    }
  }

  const handleStopGeneration = async () => {
    if (!canStop) {
      return
    }

    setIsStopping(true)
    setNotice({
      tone: 'neutral',
      text: t.generation.stopRequested,
    })

    try {
      await invoke('stop_generation')
    } catch (error) {
      setIsStopping(false)
      setNotice({
        tone: 'error',
        text: formatGenerationFailure(lang, getErrorMessage(error)),
      })
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
      text: formatImportInProgress(lang, emails.length),
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
            ? formatImportPartialSuccess(lang, result.imported, result.failed)
            : formatImportSuccess(lang, result.imported),
      })
    } catch (error) {
      setNotice({
        tone: 'error',
        text: formatImportFailure(lang, getErrorMessage(error)),
      })
    } finally {
      setIsImporting(false)
    }
  }

  const currentEmail =
    progress.email ||
    (progress.current > 0 ? t.generation.noEmailThisRound : t.generation.waitingForTask)

  return (
    <main className="app-shell">
      <section className="app-frame">
        <header className="frame-header">
          <div className="frame-title">
            <img src={logo} alt="Codexs" className="app-logo" />
            <div>
              <p className="frame-kicker">{t.app.subtitle}</p>
              <h1>{t.app.title}</h1>
            </div>
          </div>

          <div className="header-controls">
            <select
              className="lang-selector"
              value={lang}
              onChange={(e) => setLang(e.target.value as Language)}
            >
              <option value="zh">中文</option>
              <option value="en">English</option>
              <option value="ja">日本語</option>
            </select>

            <div className={`runtime-indicator ${runtimeReady ? 'is-live' : ''}`}>
              <span className="runtime-dot" />
              {runtimeReady ? t.app.runtimeConnected : t.app.browserPreview}
            </div>
          </div>
        </header>

        <div className={`status-banner tone-${notice.tone}`}>
          <span className="status-marker" />
          <span>{notice.text}</span>
        </div>

        <section className="overview-grid">
          <article className="panel">
            <div className="panel-heading">
              <p className="panel-tag">{t.generation.subtitle}</p>
              <h2>{t.generation.title}</h2>
            </div>

            <label className="field-card" htmlFor={countId}>
              <span className="field-label">{t.generation.countLabel}</span>
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
                <span className="field-suffix">{t.generation.countUnit}</span>
              </div>
            </label>

            {isGenerating ? (
              <button
                className="action-button primary"
                type="button"
                onClick={handleStopGeneration}
                disabled={!canStop}
              >
                {isStopping ? t.generation.stopping : t.generation.stopButton}
              </button>
            ) : (
              <button
                className="action-button primary"
                type="button"
                onClick={handleGenerate}
                disabled={!canGenerate}
              >
                {t.generation.startButton}
              </button>
            )}

            <div className="meta-row">
              <span>{isHydrating ? t.generation.loadingAccounts : `${accounts.length} ${t.generation.accountsLoaded}`}</span>
              <span>{requestedCount > 0 ? `${t.generation.planCount} ${requestedCount} ${t.generation.countUnit}` : t.generation.enterValidCount}</span>
            </div>
          </article>

          <article className="panel">
            <div className="panel-heading">
              <p className="panel-tag">{t.progress.subtitle}</p>
              <h2>{t.progress.title}</h2>
            </div>

            <div className="progress-summary">
              <div>
                <strong>{`${progress.current}/${totalProgress || 0}`}</strong>
                <span>{progressPercent}%</span>
              </div>
              <span className={`progress-state ${isGenerating ? 'is-active' : ''}`}>
                {isGenerating
                  ? isStopping
                    ? t.generation.stopping
                    : t.generation.inProgress
                  : generationStopped
                    ? t.generation.stopped
                  : totalProgress > 0 && progress.current >= totalProgress
                    ? t.generation.completed
                    : t.generation.idle}
              </span>
            </div>

            <div className="progress-track" aria-hidden="true">
              <div
                className="progress-fill"
                style={{ width: `${progressPercent}%` }}
              />
            </div>

            <div className="progress-card">
              <span className="progress-label">{t.generation.currentLabel}</span>
              <strong className="progress-email">{currentEmail}</strong>
            </div>

            {progressLogs.length > 0 && (
              <div className="progress-logs">
                <p>{t.generation.recentErrors}</p>
                <ul>
                  {progressLogs.slice(-5).reverse().map((log, index) => (
                    <li key={`${log.timestamp}-${index}`} className={`log-${log.type}`}>
                      {log.message}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {recentErrors.length > 0 ? (
              <div className="diagnostics">
                <p>{t.generation.recentErrors}</p>
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
              <p className="panel-tag">{t.accounts.title}</p>
              <h2>{`${t.accounts.title} (${accounts.length})`}</h2>
            </div>

            <div className="button-group">
              <button
                className="action-button tertiary"
                type="button"
                onClick={selectAllPending}
                disabled={isGenerating || isImporting || pendingCount === 0}
              >
                {t.accounts.selectAllPending}
              </button>
              <button
                className="action-button tertiary"
                type="button"
                onClick={clearSelection}
                disabled={isGenerating || isImporting || selectedCount === 0}
              >
                {t.accounts.clearSelection}
              </button>
              <button
                className="action-button secondary"
                type="button"
                onClick={handleImport}
                disabled={!canImport}
              >
                {isImporting ? t.accounts.importing : `${t.accounts.importButton}${selectedCount > 0 ? ` (${selectedCount})` : ''}`}
              </button>
            </div>
          </div>

          <div className="summary-chips" aria-label="账号统计">
            <span>{`${t.accounts.imported} ${importedCount}`}</span>
            <span>{`${t.accounts.pending} ${pendingCount}`}</span>
            <span>{`${t.accounts.selected} ${selectedCount}`}</span>
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
                      {account.imported ? t.accounts.imported : t.accounts.pending}
                    </span>
                  </label>
                )
              })}
            </div>
          ) : (
            <div className="empty-state">
              <strong>{t.accounts.emptyTitle}</strong>
              <p>{t.accounts.emptyDescription}</p>
            </div>
          )}
        </section>

        <section className="panel" style={{ marginTop: '2rem' }}>
          <div className="panel-heading">
            <h2>账号管理</h2>
          </div>
          <AccountManagement />
        </section>
      </section>
    </main>
  )
}

export default App
