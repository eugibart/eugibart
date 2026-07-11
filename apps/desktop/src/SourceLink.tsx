import { openUrl } from "@tauri-apps/plugin-opener";

/**
 * A community-citation link: "Source: ducatimonster.org ↗", opened in the
 * system browser via the (capability-scoped) opener plugin.
 *
 * Defense in depth on top of the Tauri capability: this component only ever
 * receives URLs that ship inside the compile-time-embedded guidance TOML
 * (already https-validated by the ecu-defs validator), and it re-checks the
 * scheme here so a future data-path mistake can't turn it into an open
 * redirect. Never pass user input.
 */
export default function SourceLink({ site, url }: { site: string; url: string }) {
  if (!url.startsWith("https://")) return null;
  return (
    <button
      type="button"
      className="source-link"
      onClick={() => openUrl(url).catch(() => {})}
      title={url}
    >
      Source: {site} <span aria-hidden="true">↗</span>
      <span className="visually-hidden">(opens in your browser)</span>
    </button>
  );
}
