export type Language = 'zh' | 'en' | 'ja'

export interface Translations {
  app: {
    title: string
    subtitle: string
    runtimeConnected: string
    browserPreview: string
    browserPreviewNotice: string
  }
  generation: {
    title: string
    subtitle: string
    countLabel: string
    countUnit: string
    startButton: string
    stopButton: string
    generating: string
    stopping: string
    accountsLoaded: string
    loadingAccounts: string
    planCount: string
    enterValidCount: string
    ready: string
    inProgress: string
    completed: string
    idle: string
    currentLabel: string
    waitingForTask: string
    noEmailThisRound: string
    recentErrors: string
    stopRequested: string
    stopped: string
  }
  progress: {
    title: string
    subtitle: string
  }
  accounts: {
    title: string
    importButton: string
    importing: string
    imported: string
    pending: string
    selected: string
    emptyTitle: string
    emptyDescription: string
    selectAllPending: string
    clearSelection: string
  }
  management: {
    title: string
  }
  status: {
    success: string
    generating: string
    failed: string
  }
}

const translations: Record<Language, Translations> = {
  zh: {
    app: {
      title: 'codexs',
      subtitle: 'Minimal Tech Console',
      runtimeConnected: 'Tauri 已连接',
      browserPreview: '浏览器预览',
      browserPreviewNotice: '当前是浏览器预览模式，Tauri commands 仅在桌面端可用。',
    },
    generation: {
      title: '批量生成账号',
      subtitle: '生成配置',
      countLabel: '数量',
      countUnit: '个',
      startButton: '开始生成',
      stopButton: '停止',
      generating: '生成中...',
      stopping: '停止中...',
      accountsLoaded: '个账号已载入',
      loadingAccounts: '读取账号列表中...',
      planCount: '本次计划',
      enterValidCount: '请输入有效数量',
      ready: '准备就绪，等待开始生成。',
      inProgress: '生成中',
      completed: '已完成',
      idle: '待命',
      currentLabel: '当前',
      waitingForTask: '等待生成任务',
      noEmailThisRound: '本轮未产出邮箱',
      recentErrors: '最近异常',
      stopRequested: '已请求停止，当前轮次结束后会停止。',
      stopped: '已停止',
    },
    progress: {
      title: '生成进度',
      subtitle: '进度显示',
    },
    accounts: {
      title: '账号列表',
      importButton: '导入选中账号',
      importing: '导入中...',
      imported: '已导入',
      pending: '未导入',
      selected: '已选中',
      emptyTitle: '还没有账号',
      emptyDescription: '先在上方设置数量并开始生成，列表会自动刷新。',
      selectAllPending: '全选未导入',
      clearSelection: '取消全选',
    },
    management: {
      title: '账号管理',
    },
    status: {
      success: '成功',
      generating: '正在生成',
      failed: '失败',
    },
  },
  en: {
    app: {
      title: 'codexs',
      subtitle: 'Minimal Tech Console',
      runtimeConnected: 'Tauri Connected',
      browserPreview: 'Browser Preview',
      browserPreviewNotice: 'Browser preview mode. Tauri commands are only available in desktop app.',
    },
    generation: {
      title: 'Batch Generate Accounts',
      subtitle: 'Generation Config',
      countLabel: 'Count',
      countUnit: '',
      startButton: 'Start Generation',
      stopButton: 'Stop',
      generating: 'Generating...',
      stopping: 'Stopping...',
      accountsLoaded: 'accounts loaded',
      loadingAccounts: 'Loading accounts...',
      planCount: 'Planned',
      enterValidCount: 'Enter valid count',
      ready: 'Ready to start generation.',
      inProgress: 'In Progress',
      completed: 'Completed',
      idle: 'Idle',
      currentLabel: 'Current',
      waitingForTask: 'Waiting for task',
      noEmailThisRound: 'No email this round',
      recentErrors: 'Recent Errors',
      stopRequested: 'Stop requested. The current attempt will finish first.',
      stopped: 'Stopped',
    },
    progress: {
      title: 'Generation Progress',
      subtitle: 'Progress Display',
    },
    accounts: {
      title: 'Account List',
      importButton: 'Import Selected',
      importing: 'Importing...',
      imported: 'Imported',
      pending: 'Pending',
      selected: 'Selected',
      emptyTitle: 'No accounts yet',
      emptyDescription: 'Set count above and start generation. List will refresh automatically.',
      selectAllPending: 'Select All Pending',
      clearSelection: 'Clear Selection',
    },
    management: {
      title: 'Account Management',
    },
    status: {
      success: 'Success',
      generating: 'Generating',
      failed: 'Failed',
    },
  },
  ja: {
    app: {
      title: 'codexs',
      subtitle: 'Minimal Tech Console',
      runtimeConnected: 'Tauri 接続済み',
      browserPreview: 'ブラウザプレビュー',
      browserPreviewNotice: 'ブラウザプレビューモードです。Tauri コマンドはデスクトップアプリでのみ利用可能です。',
    },
    generation: {
      title: 'アカウント一括生成',
      subtitle: '生成設定',
      countLabel: '数量',
      countUnit: '個',
      startButton: '生成開始',
      stopButton: '停止',
      generating: '生成中...',
      stopping: '停止中...',
      accountsLoaded: '個のアカウントを読み込みました',
      loadingAccounts: 'アカウントリストを読み込み中...',
      planCount: '今回の予定',
      enterValidCount: '有効な数量を入力してください',
      ready: '準備完了。生成開始を待っています。',
      inProgress: '生成中',
      completed: '完了',
      idle: '待機中',
      currentLabel: '現在',
      waitingForTask: '生成タスクを待っています',
      noEmailThisRound: 'このラウンドではメールが生成されませんでした',
      recentErrors: '最近のエラー',
      stopRequested: '停止を受け付けました。現在の試行完了後に停止します。',
      stopped: '停止済み',
    },
    progress: {
      title: '生成進捗',
      subtitle: '進捗表示',
    },
    accounts: {
      title: 'アカウントリスト',
      importButton: '選択したアカウントをインポート',
      importing: 'インポート中...',
      imported: 'インポート済み',
      pending: '未インポート',
      selected: '選択済み',
      emptyTitle: 'アカウントがありません',
      emptyDescription: '上で数量を設定して生成を開始してください。リストは自動的に更新されます。',
      selectAllPending: '未インポートを全選択',
      clearSelection: '選択解除',
    },
    management: {
      title: 'アカウント管理',
    },
    status: {
      success: '成功',
      generating: '生成中',
      failed: '失敗',
    },
  },
}

export function getTranslations(lang: Language): Translations {
  return translations[lang]
}

export function getBrowserLanguage(): Language {
  const browserLang = navigator.language.toLowerCase()
  if (browserLang.startsWith('zh')) return 'zh'
  if (browserLang.startsWith('ja')) return 'ja'
  return 'en'
}

// Helper functions for dynamic messages
export function formatGenerationSuccess(lang: Language, succeeded: number): string {
  const t = translations[lang]
  if (lang === 'zh') return `生成完成，已新增 ${succeeded} ${t.generation.countUnit}账号。`
  if (lang === 'ja') return `生成完了。${succeeded} 個のアカウントを追加しました。`
  return `Generation completed. ${succeeded} accounts added.`
}

export function formatGenerationPartialSuccess(lang: Language, succeeded: number, failed: number): string {
  const t = translations[lang]
  if (lang === 'zh') return `生成完成，成功 ${succeeded} ${t.generation.countUnit}，失败 ${failed} ${t.generation.countUnit}。`
  if (lang === 'ja') return `生成完了。成功 ${succeeded} 個、失敗 ${failed} 個。`
  return `Generation completed. ${succeeded} succeeded, ${failed} failed.`
}

export function formatGenerationStopped(lang: Language, succeeded: number, failed: number): string {
  const t = translations[lang]
  if (lang === 'zh') return `生成已停止，成功 ${succeeded} ${t.generation.countUnit}，失败 ${failed} ${t.generation.countUnit}。`
  if (lang === 'ja') return `生成を停止しました。成功 ${succeeded} 個、失敗 ${failed} 個。`
  return `Generation stopped. ${succeeded} succeeded, ${failed} failed.`
}

export function formatGenerationFailure(lang: Language, message: string): string {
  if (lang === 'zh') return `生成失败：${message}`
  if (lang === 'ja') return `生成失敗：${message}`
  return `Generation failed: ${message}`
}

export function formatInitError(lang: Language, message: string): string {
  if (lang === 'zh') return `初始化失败：${message}`
  if (lang === 'ja') return `初期化失敗：${message}`
  return `Initialization failed: ${message}`
}

export function formatImportSuccess(lang: Language, count: number): string {
  const t = translations[lang]
  if (lang === 'zh') return `导入完成，已处理 ${count} ${t.generation.countUnit}账号。`
  if (lang === 'ja') return `インポート完了。${count} 個のアカウントを処理しました。`
  return `Import completed. ${count} accounts processed.`
}

export function formatImportPartialSuccess(lang: Language, imported: number, failed: number): string {
  const t = translations[lang]
  if (lang === 'zh') return `导入完成，已处理 ${imported} ${t.generation.countUnit}，失败 ${failed} ${t.generation.countUnit}。`
  if (lang === 'ja') return `インポート完了。処理済み ${imported} 個、失敗 ${failed} 個。`
  return `Import completed. ${imported} processed, ${failed} failed.`
}

export function formatImportFailure(lang: Language, message: string): string {
  if (lang === 'zh') return `导入失败：${message}`
  if (lang === 'ja') return `インポート失敗：${message}`
  return `Import failed: ${message}`
}

export function formatImportInProgress(lang: Language, count: number): string {
  const t = translations[lang]
  if (lang === 'zh') return `正在导入 ${count} ${t.generation.countUnit}选中账号...`
  if (lang === 'ja') return `${count} 個の選択したアカウントをインポート中...`
  return `Importing ${count} selected accounts...`
}
