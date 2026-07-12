import { ReactNode, useEffect, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

/**
 * A file path the app produced, with the two actions a desktop user
 * actually wants: copy it, or see the file in the system file manager.
 * Rendering a bare path and making the user retype it is a web habit.
 */
export default function PathBox({
  path,
  suffix,
  children,
}: {
  path: string;
  /** Extra text after the path (e.g. "— 124 events so far"). */
  suffix?: ReactNode;
  children: ReactNode;
}) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(path);
      setCopied(true);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setCopied(false), 2500);
    } catch {
      // Clipboard unavailable: leave the path selectable, no fake success.
    }
  };

  return (
    <div className="ok-box path-box" role="status">
      <span className="path-box-text">
        {children} <span className="mono">{path}</span>
        {suffix}
      </span>
      <span className="path-box-actions">
        <button className="btn btn-small" onClick={copy}>
          {copied ? "Copied ✓" : "Copy path"}
        </button>
        <button
          className="btn btn-small"
          onClick={() => revealItemInDir(path).catch(() => {})}
        >
          Show in folder
        </button>
      </span>
    </div>
  );
}
