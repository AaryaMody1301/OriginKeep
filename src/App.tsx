import { FormEvent, useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type DownloadRecord = {
  id: number;
  captureKey: string;
  originalUrl: string;
  finalUrl: string | null;
  referrer: string | null;
  localPath: string;
  fileName: string;
  mimeType: string | null;
  bytes: number | null;
  startedAt: string | null;
  completedAt: string | null;
  sha256: string | null;
  status: string;
  updatedAt: string;
};

function formatBytes(bytes: number | null) {
  if (bytes === null || bytes < 0) return "Unknown size";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
}

function shortHash(hash: string | null) {
  return hash ? `${hash.slice(0, 12)}…` : "Pending fingerprint";
}

export default function App() {
  const [downloads, setDownloads] = useState<DownloadRecord[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadDownloads = useCallback(async (search = "") => {
    setLoading(true);
    setError(null);
    try {
      const rows = await invoke<DownloadRecord[]>("list_downloads", {
        query: search.trim() || null,
      });
      setDownloads(rows);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadDownloads();
  }, [loadDownloads]);

  function submitSearch(event: FormEvent) {
    event.preventDefault();
    void loadDownloads(query);
  }

  return (
    <main className="shell">
      <header className="hero">
        <div>
          <p className="eyebrow">Local-first download provenance</p>
          <h1>OriginKeep</h1>
          <p className="tagline">Downloads that remember where they came from.</p>
        </div>
        <div className="phase-chip">Phase 1 · Provenance</div>
      </header>

      <section className="summary" aria-label="Tracked download summary">
        <div>
          <strong>{downloads.length}</strong>
          <span>visible records</span>
        </div>
        <div>
          <strong>{downloads.filter((item) => item.sha256).length}</strong>
          <span>fingerprinted</span>
        </div>
        <div>
          <strong>{downloads.filter((item) => item.referrer).length}</strong>
          <span>with referrer evidence</span>
        </div>
      </section>

      <form className="search" onSubmit={submitSearch}>
        <label htmlFor="search-input">Search provenance</label>
        <div className="search-row">
          <input
            id="search-input"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filename, source URL, referrer, hash…"
          />
          <button type="submit">Search</button>
          <button type="button" className="secondary" onClick={() => void loadDownloads(query)}>
            Refresh
          </button>
        </div>
      </form>

      {error ? <p className="error">Could not read the local provenance database: {error}</p> : null}

      <section className="records" aria-live="polite">
        {loading ? <p className="empty">Reading local provenance…</p> : null}
        {!loading && downloads.length === 0 ? (
          <div className="empty-card">
            <h2>No tracked downloads yet</h2>
            <p>
              Install the browser companion and native host, then complete a download. OriginKeep will
              record source metadata locally and fingerprint the file when it is available.
            </p>
          </div>
        ) : null}

        {downloads.map((item) => (
          <article className="record" key={item.captureKey}>
            <div className="record-title">
              <div>
                <h2>{item.fileName}</h2>
                <p className="path">{item.localPath}</p>
              </div>
              <span className="status">{item.status.replaceAll("_", " ")}</span>
            </div>
            <dl>
              <div>
                <dt>Origin</dt>
                <dd>{item.originalUrl}</dd>
              </div>
              <div>
                <dt>Final URL</dt>
                <dd>{item.finalUrl || "Not reported"}</dd>
              </div>
              <div>
                <dt>Referrer</dt>
                <dd>{item.referrer || "Not reported by the browser"}</dd>
              </div>
              <div>
                <dt>Integrity</dt>
                <dd>{shortHash(item.sha256)}</dd>
              </div>
              <div>
                <dt>Size</dt>
                <dd>{formatBytes(item.bytes)}</dd>
              </div>
              <div>
                <dt>MIME</dt>
                <dd>{item.mimeType || "Unknown"}</dd>
              </div>
            </dl>
          </article>
        ))}
      </section>
    </main>
  );
}
