export interface Account {
  email: string
  created_at: string
  imported: boolean
}

export interface GeneratedAccount {
  email: string
  created_at: string
  token_path: string
  codex_token_path: string
}

export interface GenerationProgressEvent {
  current: number
  total: number
  email: string
}

export interface GenerationResult {
  requested: number
  succeeded: number
  failed: number
  accounts: GeneratedAccount[]
  errors: string[]
}

export interface ImportResult {
  requested: number
  imported: number
  skipped: number
  failed: number
  emails: string[]
}

export interface NoticeState {
  tone: 'neutral' | 'success' | 'error'
  text: string
}

export interface ProgressLog {
  type: 'success' | 'error' | 'info'
  message: string
  timestamp: string
}
