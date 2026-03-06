export interface ClickableItem {
  id: string;
  label: string;
  app_name: string;
  is_stale: boolean;
}

export interface OverlayContent {
  markdown: string;
  items: ClickableItem[];
  gemini_summary: string | null;
}
