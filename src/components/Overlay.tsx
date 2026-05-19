import { useWindowState } from "../hooks/useWindowState";
import { ActivityList } from "./ActivityList";
import { toggleContext } from "../services/tauriBridge";

export function Overlay() {
  const content = useWindowState();

  const handleToggle = async () => {
    await toggleContext();
  };

  return (
    <div className="overlay-card">
      <div className="context-toggle-row">
        <button
          className={`context-toggle ${content.context_enabled ? "on" : "off"}`}
          onClick={handleToggle}
          title={content.context_enabled ? "Deep context ON — click to disable" : "Deep context OFF — click to enable"}
        >
          <span className="toggle-icon">{content.context_enabled ? "◉" : "◎"}</span>
          <span className="toggle-label">{content.context_enabled ? "context on" : "context off"}</span>
        </button>
      </div>
      <ActivityList content={content} />
    </div>
  );
}
