import { invoke } from '@tauri-apps/api/core'
import { useEffect, useState } from 'react'
import type { Language } from '../i18n'
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

interface SshServer {
  id: string
  name: string
  host: string
  port: number
  username: string
  auth_method: string
  key_path: string | null
}

const i18n = {
  zh: {
    total: '总计',
    active: '当前',
    autoResume: '切换时自动恢复 CLI 会话',
    sshSync: 'SSH 同步',
    sshSyncHint: '切换账号时自动同步 auth.json 到远程服务器',
    sshServers: 'SSH 服务器',
    sshServersHint: '配置远程服务器用于同步',
    addServer: '添加服务器',
    importFromConfig: '从 ~/.ssh/config 导入',
    serverName: '名称',
    serverHost: '主机',
    serverPort: '端口',
    serverUsername: '用户名',
    serverKeyPath: '密钥路径',
    save: '保存',
    cancel: '取消',
    delete: '删除',
    noServers: '还没有配置服务器',
    syncNow: '立即同步',
    syncing: '同步中...',
    syncSuccess: '同步成功',
    syncFailed: '同步失败',
    refreshAll: '刷新全部',
    smartSwitch: '智能切换',
    smartSwitching: '切换中...',
    smartSwitchNoTarget: '没有可切换的账号',
    smartSwitchAlreadyBest: '当前账号已是最优（1week 优先，其次 5h）',
    loading: '加载账号中...',
    emptyTitle: '还没有账号',
    emptyHint: '请先在 Codex Tools 中添加账号',
    switch: '切换',
    deleteConfirm: '确定要删除这个账号吗？',
    deleteServerConfirm: '确定要删除这个服务器吗？',
    collapse: '收起',
    expand: '展开',
  },
  en: {
    total: 'TOTAL',
    active: 'ACTIVE',
    autoResume: 'Auto-resume CLI sessions on switch',
    sshSync: 'SSH Sync',
    sshSyncHint: 'Auto-sync auth.json to remote servers on account switch',
    sshServers: 'SSH Servers',
    sshServersHint: 'Configure remote servers for syncing',
    addServer: 'Add Server',
    importFromConfig: 'Import from ~/.ssh/config',
    serverName: 'Name',
    serverHost: 'Host',
    serverPort: 'Port',
    serverUsername: 'Username',
    serverKeyPath: 'Key Path',
    save: 'Save',
    cancel: 'Cancel',
    delete: 'Delete',
    noServers: 'No servers configured',
    syncNow: 'Sync Now',
    syncing: 'Syncing...',
    syncSuccess: 'Sync successful',
    syncFailed: 'Sync failed',
    refreshAll: 'Refresh All',
    smartSwitch: 'Smart Switch',
    smartSwitching: 'Switching...',
    smartSwitchNoTarget: 'No switchable account available',
    smartSwitchAlreadyBest: 'Current account already has the best remaining usage',
    loading: 'Loading accounts...',
    emptyTitle: 'No accounts yet',
    emptyHint: 'Add accounts in Codex Tools first',
    switch: 'Switch',
    deleteConfirm: 'Are you sure you want to delete this account?',
    deleteServerConfirm: 'Are you sure you want to delete this server?',
    collapse: 'Collapse',
    expand: 'Expand',
  },
  ja: {
    total: '合計',
    active: '現在',
    autoResume: '切り替え時に CLI セッションを自動再開',
    sshSync: 'SSH 同期',
    sshSyncHint: 'アカウント切り替え時に auth.json をリモートサーバーに自動同期',
    sshServers: 'SSH サーバー',
    sshServersHint: '同期用のリモートサーバーを設定',
    addServer: 'サーバーを追加',
    importFromConfig: '~/.ssh/config からインポート',
    serverName: '名前',
    serverHost: 'ホスト',
    serverPort: 'ポート',
    serverUsername: 'ユーザー名',
    serverKeyPath: '鍵パス',
    save: '保存',
    cancel: 'キャンセル',
    delete: '削除',
    noServers: 'サーバーが設定されていません',
    syncNow: '今すぐ同期',
    syncing: '同期中...',
    syncSuccess: '同期成功',
    syncFailed: '同期失敗',
    refreshAll: 'すべて更新',
    smartSwitch: 'スマート切替',
    smartSwitching: '切り替え中...',
    smartSwitchNoTarget: '切り替え可能なアカウントがありません',
    smartSwitchAlreadyBest: '現在のアカウントが最適です',
    loading: 'アカウントを読み込み中...',
    emptyTitle: 'アカウントがありません',
    emptyHint: '先に Codex Tools でアカウントを追加してください',
    switch: '切り替え',
    deleteConfirm: 'このアカウントを削除してもよろしいですか？',
    deleteServerConfirm: 'このサーバーを削除してもよろしいですか？',
    collapse: '折りたたむ',
    expand: '展開',
  },
}

function windowRemaining(window: UsageWindow | null | undefined): number {
  if (!window || window.used_percent === null || window.used_percent === undefined) return -1
  return Math.max(0, Math.min(100, 100 - window.used_percent))
}

function pickBestAccount(accounts: ManagedAccount[]): ManagedAccount | null {
  if (accounts.length === 0) return null
  const sorted = [...accounts].sort((a, b) => {
    const aWeek = windowRemaining(a.usage?.one_week)
    const bWeek = windowRemaining(b.usage?.one_week)
    if (bWeek !== aWeek) return bWeek - aWeek
    const aHour = windowRemaining(a.usage?.five_hour)
    const bHour = windowRemaining(b.usage?.five_hour)
    if (bHour !== aHour) return bHour - aHour
    if (a.is_current !== b.is_current) return a.is_current ? -1 : 1
    return (a.label || '').localeCompare(b.label || '')
  })
  return sorted[0] ?? null
}

function usageBarColor(percent: number): string {
  if (percent > 90) return '#ef4444'
  if (percent > 70) return '#f59e0b'
  return '#10b981'
}

export function AccountManagement({ lang = 'zh' }: { lang?: Language }) {
  const t = i18n[lang]
  const [accounts, setAccounts] = useState<ManagedAccount[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState<string | null>(null)
  const [autoResume, setAutoResume] = useState(true)
  const [sshAutoSync, setSshAutoSync] = useState(false)
  const [sshServers, setSshServers] = useState<SshServer[]>([])
  const [showServerForm, setShowServerForm] = useState(false)
  const [showImportList, setShowImportList] = useState(false)
  const [importableServers, setImportableServers] = useState<SshServer[]>([])
  const [editingServer, setEditingServer] = useState<Partial<SshServer>>({
    name: '',
    host: '',
    port: 22,
    username: '',
    key_path: null,
  })
  const [syncing, setSyncing] = useState(false)
  const [smartSwitching, setSmartSwitching] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const [collapsed, setCollapsed] = useState(false)
  const [sshCollapsed, setSshCollapsed] = useState(false)

  useEffect(() => {
    loadAccounts()
    loadSshAutoSync()
    loadSshServers()

    // 每30秒自动刷新用量
    const usageTimer = setInterval(() => {
      loadAccounts()
    }, 30000)

    return () => {
      clearInterval(usageTimer)
    }
  }, [])

  useEffect(() => {
    if (!notice) return
    const timer = setTimeout(() => setNotice(null), 3000)
    return () => clearTimeout(timer)
  }, [notice])

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

  async function loadSshAutoSync() {
    try {
      const enabled = await invoke<boolean>('get_ssh_auto_sync')
      setSshAutoSync(enabled)
    } catch (error) {
      console.error('Failed to load SSH auto sync:', error)
    }
  }

  async function handleSshAutoSyncChange(enabled: boolean) {
    try {
      await invoke('set_ssh_auto_sync', { enabled })
      setSshAutoSync(enabled)
    } catch (error) {
      console.error('Failed to set SSH auto sync:', error)
    }
  }

  async function handleSyncNow() {
    try {
      setSyncing(true)
      const results = await invoke<string[]>('sync_auth_to_ssh')
      if (results.length === 0) {
        setNotice(lang === 'zh' ? '没有配置 SSH 服务器' : lang === 'ja' ? 'SSH サーバーが設定されていません' : 'No SSH servers configured')
      } else if (results.every(r => r.startsWith('✓'))) {
        setNotice(t.syncSuccess)
      } else {
        setNotice(t.syncFailed + ': ' + results.filter(r => r.startsWith('✗')).join(', '))
      }
    } catch (error) {
      console.error('Failed to sync:', error)
      setNotice(t.syncFailed)
    } finally {
      setSyncing(false)
    }
  }

  async function handleSwitch(id: string) {
    try {
      console.log('=== 切换账号 ===')
      console.log('账号ID:', id)
      console.log('autoResume:', autoResume)
      await invoke('switch_account', { id, autoResume })
      console.log('切换成功')
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
    if (!confirm(t.deleteConfirm)) return
    try {
      await invoke('delete_account', { id })
      await loadAccounts()
    } catch (error) {
      console.error('Failed to delete account:', error)
    }
  }

  async function handleSmartSwitch() {
    if (smartSwitching) return
    const target = pickBestAccount(accounts)
    if (!target) { setNotice(t.smartSwitchNoTarget); return }
    if (target.is_current) { setNotice(t.smartSwitchAlreadyBest); return }
    try {
      setSmartSwitching(true)
      await invoke('switch_account', { id: target.id, autoResume })
      await loadAccounts()
    } catch (error) {
      console.error('Smart switch failed:', error)
    } finally {
      setSmartSwitching(false)
    }
  }

  async function loadSshServers() {
    try {
      console.log('Loading SSH servers...')
      const servers = await invoke<SshServer[]>('list_ssh_servers')
      console.log('Loaded SSH servers:', servers)
      setSshServers(servers)
    } catch (error) {
      console.error('Failed to load SSH servers:', error)
    }
  }

  async function handleImportFromConfig() {
    try {
      const servers = await invoke<SshServer[]>('parse_ssh_config')
      setImportableServers(servers)
      setShowImportList(true)
    } catch (error) {
      console.error('Failed to parse SSH config:', error)
      setNotice(lang === 'zh' ? '解析 SSH config 失败' : lang === 'ja' ? 'SSH config の解析に失敗しました' : 'Failed to parse SSH config')
    }
  }

  async function handleImportServer(server: SshServer) {
    try {
      await invoke('add_ssh_server', { server })
      await loadSshServers()
      setShowImportList(false)
      setImportableServers([])
    } catch (error) {
      console.error('Failed to import server:', error)
    }
  }

  async function handleSaveServer() {
    if (!editingServer.name || !editingServer.host || !editingServer.username) {
      setNotice(lang === 'zh' ? '请填写完整信息' : lang === 'ja' ? '情報を入力してください' : 'Please fill in all fields')
      return
    }
    try {
      const server: SshServer = {
        id: editingServer.id || '',
        name: editingServer.name,
        host: editingServer.host,
        port: editingServer.port || 22,
        username: editingServer.username,
        auth_method: 'key',
        key_path: editingServer.key_path || null,
      }
      await invoke('add_ssh_server', { server })
      await loadSshServers()
      setShowServerForm(false)
      setEditingServer({ name: '', host: '', port: 22, username: '', key_path: null })
    } catch (error) {
      console.error('Failed to save server:', error)
      setNotice(t.syncFailed)
    }
  }

  async function handleDeleteServer(id: string) {
    console.log('=== DELETE BUTTON CLICKED ===')
    console.log('Server ID:', id)
    console.log('Current servers:', sshServers)

    try {
      console.log('Calling delete_ssh_server...')
      await invoke('delete_ssh_server', { id })
      console.log('Delete successful, reloading servers...')
      await loadSshServers()
      setNotice(lang === 'zh' ? '删除成功' : lang === 'ja' ? '削除成功' : 'Deleted successfully')
    } catch (error) {
      console.error('Failed to delete server:', error)
      setNotice(lang === 'zh' ? '删除失败: ' + error : lang === 'ja' ? '削除失敗: ' + error : 'Delete failed: ' + error)
    }
  }


  if (loading) {
    return (
      <div className="account-management">
        <div className="loading-state">
          <div className="spinner" />
          <p>{t.loading}</p>
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
            <span className="stat-label">{t.total}</span>
            <span className="stat-value">{accounts.length}</span>
          </div>
          {currentAccount && (
            <div className="stat current">
              <span className="stat-label">{t.active}</span>
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
            <span>{t.autoResume}</span>
          </label>
          <label className="toggle-control" title={t.sshSyncHint}>
            <input
              type="checkbox"
              checked={sshAutoSync}
              onChange={(e) => handleSshAutoSyncChange(e.target.checked)}
            />
            <span>{t.sshSync}</span>
          </label>
          <button
            className="btn-sync"
            onClick={handleSyncNow}
            disabled={syncing}
          >
            {syncing ? t.syncing : t.syncNow}
          </button>
          <button
            className="btn-smart-switch"
            onClick={handleSmartSwitch}
            disabled={smartSwitching || accounts.length === 0}
          >
            {smartSwitching ? t.smartSwitching : t.smartSwitch}
          </button>
          <button className="btn-refresh" onClick={loadAccounts}>
            ⟳ {t.refreshAll}
          </button>
          <button className="btn-collapse" onClick={() => setCollapsed(!collapsed)}>
            {collapsed ? t.expand : t.collapse}
          </button>
        </div>
      </div>

      {notice && <div className="smart-notice">{notice}</div>}

      {!collapsed && (
        <>
          {accounts.length === 0 ? (
            <div className="empty-state">
              <p>{t.emptyTitle}</p>
              <p className="hint">{t.emptyHint}</p>
            </div>
          ) : (
            <div className="account-table">
              <div className="table-header">
                <span className="col-email">Email</span>
                <span className="col-plan">Plan</span>
                <span className="col-usage">5H</span>
                <span className="col-usage">1W</span>
                <span className="col-actions"></span>
              </div>
              {sortedAccounts.map(account => (
                <AccountRow
                  key={account.id}
                  account={account}
                  refreshing={refreshing === account.id}
                  onSwitch={() => handleSwitch(account.id)}
                  onRefresh={() => handleRefresh(account.id)}
                  onDelete={() => handleDelete(account.id)}
                  lang={lang}
                />
              ))}
            </div>
          )}
        </>
      )}

      {/* SSH Servers Section */}
      <div className="ssh-section">
        <div className="ssh-header">
          <div>
            <h3>{t.sshServers}</h3>
            <p className="hint">{t.sshServersHint}</p>
          </div>
          <div className="ssh-controls">
            <button className="btn-add" onClick={() => setShowServerForm(true)}>
              + {t.addServer}
            </button>
            <button className="btn-import" onClick={handleImportFromConfig}>
              {t.importFromConfig}
            </button>
            <button className="btn-collapse" onClick={() => setSshCollapsed(!sshCollapsed)}>
              {sshCollapsed ? t.expand : t.collapse}
            </button>
          </div>
        </div>

        {!sshCollapsed && (
          <>
            {showServerForm && (
              <div className="server-form">
                <input
                  type="text"
                  placeholder={t.serverName}
                  value={editingServer.name || ''}
                  onChange={(e) => setEditingServer({ ...editingServer, name: e.target.value })}
                />
                <input
                  type="text"
                  placeholder={t.serverHost}
                  value={editingServer.host || ''}
                  onChange={(e) => setEditingServer({ ...editingServer, host: e.target.value })}
                />
                <input
                  type="number"
                  placeholder={t.serverPort}
                  value={editingServer.port || 22}
                  onChange={(e) => setEditingServer({ ...editingServer, port: parseInt(e.target.value) || 22 })}
                />
                <input
                  type="text"
                  placeholder={t.serverUsername}
                  value={editingServer.username || ''}
                  onChange={(e) => setEditingServer({ ...editingServer, username: e.target.value })}
                />
                <input
                  type="text"
                  placeholder={t.serverKeyPath + ' (optional)'}
                  value={editingServer.key_path || ''}
                  onChange={(e) => setEditingServer({ ...editingServer, key_path: e.target.value || null })}
                />
                <div className="form-actions">
                  <button className="btn-save" onClick={handleSaveServer}>{t.save}</button>
                  <button className="btn-cancel" onClick={() => {
                    setShowServerForm(false)
                    setEditingServer({ name: '', host: '', port: 22, username: '', key_path: null })
                  }}>{t.cancel}</button>
                </div>
              </div>
            )}

            {showImportList && importableServers.length > 0 && (
              <div className="import-list">
                {importableServers.map((server, idx) => (
                  <div key={idx} className="import-item">
                    <span>{server.name} ({server.username}@{server.host}:{server.port})</span>
                    <button className="btn-import-one" onClick={() => handleImportServer(server)}>+</button>
                  </div>
                ))}
                <button className="btn-cancel" onClick={() => {
                  setShowImportList(false)
                  setImportableServers([])
                }}>{t.cancel}</button>
              </div>
            )}

            {sshServers.length === 0 ? (
              <div className="empty-state">
                <p>{t.noServers}</p>
              </div>
            ) : (
              <div className="server-list">
                {sshServers.map(server => (
                  <div key={server.id} className="server-item">
                    <div className="server-info">
                      <span className="server-name">{server.name}</span>
                      <span className="server-detail">{server.username}@{server.host}:{server.port}</span>
                    </div>
                    <button
                      className="btn-delete-small"
                      onClick={() => void handleDeleteServer(server.id)}
                      type="button"
                    >
                      {t.delete}
                    </button>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}

interface AccountRowProps {
  account: ManagedAccount
  refreshing: boolean
  onSwitch: () => void
  onRefresh: () => void
  onDelete: () => void
  lang: Language
}

function AccountRow({ account, refreshing, onSwitch, onRefresh, onDelete, lang }: AccountRowProps) {
  const t = i18n[lang]
  const fiveH = account.usage?.five_hour.used_percent ?? -1
  const oneW = account.usage?.one_week.used_percent ?? -1

  return (
    <div className={`table-row ${account.is_current ? 'current' : ''}`}>
      <span className="col-email" title={account.email || account.label}>
        {account.is_current && <span className="dot-active">●</span>}
        {account.email || account.label}
      </span>
      <span className="col-plan">
        <span className="plan-tag">{account.plan_type || 'FREE'}</span>
      </span>
      <span className="col-usage">
        {fiveH >= 0 ? (
          <span className="mini-bar">
            <span className="mini-fill" style={{ width: `${Math.min(100, fiveH)}%`, background: usageBarColor(fiveH) }} />
            <span className="mini-text">{fiveH.toFixed(0)}%</span>
          </span>
        ) : '—'}
      </span>
      <span className="col-usage">
        {oneW >= 0 ? (
          <span className="mini-bar">
            <span className="mini-fill" style={{ width: `${Math.min(100, oneW)}%`, background: usageBarColor(oneW) }} />
            <span className="mini-text">{oneW.toFixed(0)}%</span>
          </span>
        ) : '—'}
      </span>
      <span className="col-actions">
        {!account.is_current && (
          <button className="btn-row-switch" onClick={onSwitch}>{t.switch}</button>
        )}
        <button className="btn-row-icon" onClick={onRefresh} disabled={refreshing}>
          {refreshing ? '…' : '⟳'}
        </button>
        <button className="btn-row-icon btn-row-delete" onClick={onDelete}>×</button>
      </span>
    </div>
  )
}
