import type { ClickableItem } from "../types";
import { focusWindow } from "../services/tauriBridge";

interface Props {
  item: ClickableItem;
}

export function ActivityItem({ item }: Props) {
  const handleClick = () => {
    focusWindow(item.app_name);
  };

  return (
    <button
      className={`activity-item ${item.is_stale ? "stale" : ""}`}
      onClick={handleClick}
    >
      <span className="activity-label">{item.label}</span>
    </button>
  );
}
