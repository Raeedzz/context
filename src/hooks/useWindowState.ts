import { useState, useEffect, useRef } from "react";
import type { OverlayContent } from "../types";
import { getOverlayContent } from "../services/tauriBridge";

export function useWindowState() {
  const [content, setContent] = useState<OverlayContent>({
    markdown: "*Scanning windows...*",
    items: [],
    context_enabled: false,
  });
  const lastHash = useRef("");

  useEffect(() => {
    const poll = async () => {
      try {
        const data = await getOverlayContent();
        const hash =
          data.markdown +
          data.items.map((i) => i.id + i.label).join("");
        if (hash !== lastHash.current) {
          lastHash.current = hash;
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
