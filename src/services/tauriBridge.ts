import { invoke } from "@tauri-apps/api/core";
import type { OverlayContent } from "../types";

export async function getOverlayContent(): Promise<OverlayContent> {
  return invoke<OverlayContent>("get_overlay_content");
}

export async function focusWindow(appName: string): Promise<void> {
  return invoke("focus_window", { appName });
}

export async function dismissOverlay(): Promise<void> {
  return invoke("dismiss_overlay");
}

export async function toggleContext(): Promise<boolean> {
  return invoke<boolean>("toggle_context");
}
