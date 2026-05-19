import type { OverlayContent } from "../types";
import { ActivityItem } from "./ActivityItem";

interface Props {
  content: OverlayContent;
}

export function ActivityList({ content }: Props) {
  if (content.items.length === 0 && content.markdown) {
    return <div className="empty-message">{content.markdown}</div>;
  }

  const active = content.items.filter((i) => !i.is_stale);
  const stale = content.items.filter((i) => i.is_stale);

  return (
    <div className="activity-list">
      {active.map((item) => (
        <ActivityItem key={item.id} item={item} />
      ))}

      {stale.length > 0 && (
        <>
          <div className="stale-divider">recently</div>
          {stale.map((item) => (
            <ActivityItem key={item.id} item={item} />
          ))}
        </>
      )}
    </div>
  );
}
