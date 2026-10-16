import { createMockBackend } from './mock'
import { tauriBackend } from './tauri'
import type { Backend } from './types'

export type { Backend, MediaUrls } from './types'

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** The engine backend: Rust core inside Tauri, or the in-browser mock. */
export const backend: Backend = inTauri ? tauriBackend : createMockBackend()
