import { invoke } from "@tauri-apps/api/core";
import type { AppError } from "../generated/entity/AppError";
import { useToasts } from "../app/toast";

/** 백엔드 viewmodel 명령 호출. 실패하면 AppError 메시지를 토스트로 띄우고 undefined 를 돌려준다. */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T | undefined> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    useToasts.getState().push(errorMessage(e));
    return undefined;
  }
}

export function errorMessage(e: unknown): string {
  const error = e as Partial<AppError> | string;
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") {
    return error.message;
  }
  return String(e);
}
