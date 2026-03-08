import { invoke } from '@tauri-apps/api/core'
import { useEffect, useState } from 'react'
import './AccountManagement.css'

interface ManagedAccount {
  id: string
  label: string
  email: string | null
  account_id: string
  plan_type: string | null
  added_at: number
  updated_at: number
  usage: Usage | null
  usage_error: string | null
  is_current: boolean
}

interface Usage {
  fetched_at: number
  plan_type: string | null
  five_hour: UsageWindow
  one_week: UsageWindow
  credits: Credits | null
}

interface UsageWindow {
  used_percent: number
  window_seconds: number
  reset_at: number
}

interface Credits {
  has_credits: boolean
  unlimited: boolean
  balance: number | null
}

export function AccountManagement() {
  const [accounts, setAccounts] = useState<ManagedAccount[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState<string | null>(null)
  const [autoResume, setAutoResume] = useState(true)

  useEffect(() => {
    loadAccounts()
  }, [])

  async function loadAccounts() {
    try {
      setLoading(true)
      const result = await invoke<ManagedAccount[]>('list_accounts')
      setAccounts(result)
    } catch (error) {
      console.error('Failed to load accounts:', error)
    } finally {
      setLoading(false)
    }
  }

  async function handleSwitch(id: string) {
    try {
      await invoke('switch_account', { id, autoResume })
      await loadAccounts()
    } catch (error) {
      console.error('Failed to switch account:', error)
    }
  }

  async function handleRefresh(id: string) {
    try {
      setRefreshing(id)
      const updated = await invoke<ManagedAccount>('refresh_account_usage', { id })
      setAccounts(prev => prev.map(acc => acc.id === id ? updated : acc))
    } catch (error) {
      console.error('Failed to refresh usage:', error)
    } finally {
      setRefreshing(null)
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('确定要删除这个账号吗？')) return

    try {
      await invoke('delete_account', { id })
      await loadAccounts()
    } catch (error) {
      console.error('Failed to delete account:', error)
    }
  }

  if (loading) {
    return (
      <div className="account-management">
        <div className="loading-state">
          <div className="spinner" />
          <p>Loading accounts...</p>
        </div>
      </div>
    )
  }

  const currentAccount = accounts.find(acc => acc.is_current)
  const sortedAccounts = [...accounts].sort((a, b) => {
    if (a.is_current) return -1
    if (b.is_current) return 1
    return b.updated_at - a.updated_at
  })

  return (
    <div className="account-management">
      <div className="account-header">
        <div className="header-stats">
          <div className="stat">
            <span className="stat-label">TOTAL</span>
            <span className="stat-value">{accounts.length}</span>
          </div>
          {currentAccount && (
            <div className="stat current">
              <span className="stat-label">ACTIVE</span>
              <span className="stat-value">{currentAccount.email || currentAccount.label}</span>
            </div>
          )}
        </div>

        <div className="header-controls">
          <label className="toggle-control">
            <input
              type="checkbox"
              checked={autoResume}
              onChange={(e) => setAutoResume(e.target.checked)}
            />
            <span>Auto-resume CLI sessions</span>
          </label>
          <button className="btn-refresh" onClick={loadAccounts}>
            ⟳ Refresh All
          </button>
        </div>
      </div>

      <div className="account-grid">
        {sortedAccounts.map(account => (
          <AccountCard
            key={account.id}
            account={account}
            refreshing={refreshing === account.id}
            onSwitch={() => handleSwitch(account.id)}
            onRefresh={() => handleRefresh(account.id)}
            onDelete={() => handleDelete(account.id)}
          />
        ))}
      </div>

      {accounts.length === 0 && (
        <div className="empty-state">
          <p>No accounts yet</p>
          <p className="hint">Add an account to get started</p>
        </div>
      )}
    </div>
  )
}

interface AccountCardProps {
  account: ManagedAccount
  refreshing: boolean
  onSwitch: () => void
  onRefresh: () => void
  onDelete: () => void
}

function AccountCard({ account, refreshing, onSwitch, onRefresh, onDelete }: AccountCardProps) {
  const planColor = {
    free: '#666',
    plus: '#10b981',
    pro: '#8b5cf6',
    team: '#f59e0b',
  }[account.plan_type?.toLowerCase() || 'free'] || '#666'

  return (
    <div className={`account-card ${account.is_current ? 'current' : ''}`}>
      <div className="card-header">
        <div className="account-info">
          <div className="account-email">{account.email || account.label}</div>
          <div className="account-meta">
            <span className="plan-badge" style={{ borderColor: planColor, color: planColor }}>
              {account.plan_type || 'FREE'}
            </span>
            {account.is_current && <span className="current-badge">● ACTIVE</span>}
          </div>
        </div>
      </div>

      {account.usage && (
        <div className="usage-display">
          <UsageBar
            label="5H"
            percent={account.usage.five_hour.used_percent}
            resetAt={account.usage.five_hour.reset_at}
          />
          <UsageBar
            label="1W"
            percent={account.usage.one_week.used_percent}
            resetAt={account.usage.one_week.reset_at}
          />
        </div>
      )}

      {account.usage_error && (
        <div className="usage-error">
          <span>⚠</span> {account.usage_error}
        </div>
      )}

      <div className="card-actions">
        {!account.is_current && (
          <button className="btn-switch" onClick={onSwitch}>
            Switch
          </button>
        )}
        <button
          className="btn-refresh-single"
          onClick={onRefresh}
          disabled={refreshing}
        >
          {refreshing ? '...' : '⟳'}
        </button>
        <button className="btn-delete" onClick={onDelete}>
          ×
        </button>
      </div>
    </div>
  )
}

interface UsageBarProps {
  label: string
  percent: number
  resetAt: number
}

function UsageBar({ label, percent, resetAt }: UsageBarProps) {
  const now = Date.now() / 1000
  const remaining = Math.max(0, resetAt - now)
  const hours = Math.floor(remaining / 3600)
  const minutes = Math.floor((remaining % 3600) / 60)

  const barColor = percent > 90 ? '#ef4444' : percent > 70 ? '#f59e0b' : '#10b981'

  return (
    <div className="usage-bar">
      <div className="usage-header">
        <span className="usage-label">{label}</span>
        <span className="usage-percent">{percent.toFixed(1)}%</span>
      </div>
      <div className="progress-track">
        <div
          className="progress-fill"
          style={{ width: `${Math.min(100, percent)}%`, backgroundColor: barColor }}
        />
      </div>
      <div className="usage-reset">
        Reset in {hours}h {minutes}m
      </div>
    </div>
  )
}
