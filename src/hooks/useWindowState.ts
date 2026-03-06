import { useState, useEffect, useRef } from "react";
import type { OverlayContent } from "../types";
import { getOverlayContent } from "../services/tauriBridge";

export function useWindowState() {
  const [content, setContent] = useState<OverlayContent>({
    markdown: "*Scanning windows...*",
    items: [],
    gemini_summary: null,
  });
  const lastMarkdown = useRef("");

  useEffect(() => {
    const poll = async () => {
      try {
        const data = await getOverlayContent();
        // Only update if content actually changed to prevent unnecessary re-renders
        const combined = data.markdown + (data.gemini_summary ?? "");
        if (combined !== lastMarkdown.current) {
          lastMarkdown.current = combined;
          setContent(data);
        }
      } catch {
        // Backend not ready yet
      }
    };

    poll();
    const interval = setInterval(poll, 3000);
    return () => clearInterval(interval);
  }, []);

  return content;
}
