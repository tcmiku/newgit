import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Request } from './types';

export const native = isTauri();

export async function request<T>(request: Request, repository: string | null): Promise<T> {
  if (!native) throw new Error('请在 gitpane 桌面应用中打开本地仓库。');
  return invoke<T>('git_request', { request, repository });
}

export function readSetting<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem(`gitpane:${key}`) ?? 'null') ?? fallback;
  } catch {
    return fallback;
  }
}

export function saveSetting(key: string, value: unknown) {
  try {
    localStorage.setItem(`gitpane:${key}`, JSON.stringify(value));
  } catch {
    /* Storage may be unavailable. */
  }
}
