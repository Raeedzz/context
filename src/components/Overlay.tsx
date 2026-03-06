import { useWindowState } from "../hooks/useWindowState";
import { ActivityList } from "./ActivityList";

export function Overlay() {
  const content = useWindowState();

  return (
    <div className="overlay-card">
      <div className="overlay-header">
        <span className="overlay-title">context</span>
        <span className="overlay-hint">Cmd+Shift+/ to toggle</span>
      </div>
      <ActivityList content={content} />
    </div>
  );
}
