export interface ClickableItem {
  id: string;
  label: string;
  app_name: string;
  source_type: string;
  is_stale: boolean;
}

export interface OverlayContent {
  markdown: string;
  items: ClickableItem[];
  context_enabled: boolean;
}
