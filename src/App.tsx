import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
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
  sourceIdentity: string | null;
  versionNumber: number | null;
  duplicateOfId: number | null;
  localState: string;
  updatedAt: string;
};

type VerificationSummary = {
  checked: number;
  present: number;
  modified: number;
  missing: number;
  unavailable: number;
};

type VersionFamily = {
  sourceIdentity: string;
  items: DownloadRecord[];
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

function readableState(value: string) {
  return value.replaceAll("_", " ");
}

export default function App() {
  const [downloads, setDownloads] = useState<DownloadRecord[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [verifying, setVerifying] = useState(false);
  const [verificationNote, setVerificationNote] = useState<string | null>(null);
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

  const families = useMemo<VersionFamily[]>(() => {
    const grouped = new Map<string, DownloadRecord[]>();
    for (const item of downloads) {
      if (!item.sourceIdentity || item.versionNumber === null) continue;
      const existing = grouped.get(item.sourceIdentity) ?? [];
      existing.push(item);
      grouped.set(item.sourceIdentity, existing);
    }
    return Array.from(grouped, ([sourceIdentity, items]) => ({ sourceIdentity, items })).sort(
      (left, right) => right.items.length - left.items.length,
    );
  }, [downloads]);

  function submitSearch(event: FormEvent) {
    event.preventDefault();
    void loadDownloads(query);
  }

  async function verifyLocal() {
    setVerifying(true);
    setVerificationNote(null);
    setError(null);
    try {
      const result = await invoke<VerificationSummary>("verify_local_files");
      setVerificationNote(
        `Verified ${result.checked} files: ${result.modified} modified, ${result.missing} missing, ${result.present} present${result.unavailable ? `, ${result.unavailable} unreadable` : ""}.`,
      );
      await loadDownloads(query);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setVerifying(false);
    }
  }

  return (
    <main className="shell">
      <header className="hero">
        <div>
          <p className="eyebrow">Local-first download provenance</p>
          <h1>OriginKeep</h1>
          <p className="tagline">Downloads that remember where they came from.</p>
        </div>
        <div className="phase-chip">Phase 2 · Version intelligence</div>
      </header>

      <section className="summary" aria-label="Tracked download summary">
        <div>
          <strong>{downloads.length}</strong>
          <span>visible records</span>
        </div>
        <div>
          <strong>{families.length}</strong>
          <span>version families</span>
        </div>
        <div>
          <strong>{downloads.filter((item) => item.duplicateOfId !== null).length}</strong>
          <span>exact duplicates</span>
        </div>
        <div>
          <strong>{downloads.filter((item) => item.localState === "LOCAL_MODIFIED").length}</strong>
          <span>locally modified</span>
        </div>
      </section>

      <form className="search" onSubmit={submitSearch}>
        <label htmlFor="search-input">Search provenance and version evidence</label>
        <div className="search-row">
          <input
            id="search-input"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filename, source identity, URL, referrer, hash…"
          />
          <button type="submit">Search</button>
          <button type="button" className="secondary" onClick={() => void loadDownloads(query)}>
            Refresh
          </button>
          <button type="button" className="secondary" disabled={verifying} onClick={() => void verifyLocal()}>
            {verifying ? "Verifying…" : "Verify local files"}
          </button>
        </div>
        {verificationNote ? <p className="verification-note">{verificationNote}</p> : null}
      </form>

      {error ? <p className="error">Could not read or verify local provenance: {error}</p> : null}

      {!loading && families.length > 0 ? (
        <section className="families" aria-labelledby="families-title">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Deterministic grouping</p>
              <h2 id="families-title">Version families</h2>
            </div>
            <p>Families use normalized initiating source identity; hashes decide exact content equality.</p>
          </div>

          {families.map((family) => {
            const versions = new Map<number, DownloadRecord[]>();
            for (const item of family.items) {
              if (item.versionNumber === null) continue;
              const existing = versions.get(item.versionNumber) ?? [];
              existing.push(item);
              versions.set(item.versionNumber, existing);
            }

            return (
              <article className="family-card" key={family.sourceIdentity}>
                <p className="family-source">{family.sourceIdentity}</p>
                <div className="timeline">
                  {Array.from(versions.entries())
                    .sort(([left], [right]) => right - left)
                    .map(([version, items]) => {
                      const primary = items.find((item) => item.duplicateOfId === null) ?? items[0];
                      const duplicates = items.filter((item) => item.duplicateOfId !== null).length;
                      return (
                        <div className="version-node" key={version}>
                          <span className="version-number">v{version}</span>
                          <div>
                            <strong>{primary.fileName}</strong>
                            <p>{shortHash(primary.sha256)}</p>
                            <p>
                              {readableState(primary.status)} · {readableState(primary.localState)}
                              {duplicates ? ` · ${duplicates} duplicate${duplicates === 1 ? "" : "s"}` : ""}
                            </p>
                          </div>
                        </div>
                      );
                    })}
                </div>
              </article>
            );
          })}
        </section>
      ) : null}

      <section className="records" aria-live="polite">
        {loading ? <p className="empty">Reading local provenance…</p> : null}
        {!loading && downloads.length === 0 ? (
          <div className="empty-card">
            <h2>No tracked downloads yet</h2>
            <p>
              Complete a browser download through the OriginKeep companion. Phase 2 will fingerprint the
              file, normalize its source identity, and build version evidence locally.
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
              <div className="badge-row">
                {item.versionNumber !== null ? <span className="status">v{item.versionNumber}</span> : null}
                <span className="status">{readableState(item.status)}</span>
                <span className={`status local-state ${item.localState.toLowerCase()}`}>
                  {readableState(item.localState)}
                </span>
              </div>
            </div>
            <dl>
              <div>
                <dt>Source identity</dt>
                <dd>{item.sourceIdentity || "No canonical HTTP(S) identity"}</dd>
              </div>
              <div>
                <dt>Origin</dt>
                <dd>{item.originalUrl}</dd>
              </div>
              <div>
                <dt>Final URL</dt>
                <dd>{item.finalUrl || "Not reported"}</dd>
              </div>
              <div>
                <dt>Exact duplicate</dt>
                <dd>{item.duplicateOfId === null ? "No" : `Matches record #${item.duplicateOfId}`}</dd>
              </div>
              <div>
                <dt>Integrity</dt>
                <dd>{shortHash(item.sha256)}</dd>
              </div>
              <div>
                <dt>Size / MIME</dt>
                <dd>
                  {formatBytes(item.bytes)} · {item.mimeType || "Unknown MIME"}
                </dd>
              </div>
            </dl>
          </article>
        ))}
      </section>
    </main>
  );
}
