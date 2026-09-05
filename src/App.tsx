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

type RemoteEvidence = {
  downloadId: number;
  checkedAt: string;
  requestMethod: string;
  requestUrl: string;
  finalUrl: string | null;
  httpStatus: number | null;
  resultState: string;
  etag: string | null;
  lastModified: string | null;
  contentLength: number | null;
  evidence: string;
  error: string | null;
};

type ComparisonResult = {
  currentId: number;
  previousId: number;
  kind: string;
  currentName: string;
  previousName: string;
  summary: string;
  details: string[];
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

function canCheckRemote(item: DownloadRecord) {
  return Boolean(item.sourceIdentity) && item.duplicateOfId === null && item.status !== "SUPERSEDED";
}

function canCompare(item: DownloadRecord) {
  return (
    item.versionNumber !== null &&
    item.versionNumber > 1 &&
    item.duplicateOfId === null &&
    item.localState === "PRESENT"
  );
}

export default function App() {
  const [downloads, setDownloads] = useState<DownloadRecord[]>([]);
  const [remoteEvidence, setRemoteEvidence] = useState<RemoteEvidence[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [verifying, setVerifying] = useState(false);
  const [checkingId, setCheckingId] = useState<number | null>(null);
  const [comparingId, setComparingId] = useState<number | null>(null);
  const [comparison, setComparison] = useState<ComparisonResult | null>(null);
  const [verificationNote, setVerificationNote] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadDownloads = useCallback(async (search = "") => {
    setLoading(true);
    setError(null);
    try {
      const [rows, evidence] = await Promise.all([
        invoke<DownloadRecord[]>("list_downloads", {
          query: search.trim() || null,
        }),
        invoke<RemoteEvidence[]>("list_remote_evidence"),
      ]);
      setDownloads(rows);
      setRemoteEvidence(evidence);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadDownloads();
  }, [loadDownloads]);

  const evidenceByDownload = useMemo(
    () => new Map(remoteEvidence.map((evidence) => [evidence.downloadId, evidence])),
    [remoteEvidence],
  );

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

  async function checkRemote(downloadId: number) {
    setCheckingId(downloadId);
    setError(null);
    try {
      const evidence = await invoke<RemoteEvidence>("check_remote_freshness", { downloadId });
      setVerificationNote(
        `Remote check for record #${downloadId}: ${readableState(evidence.resultState)}${
          evidence.httpStatus ? ` (HTTP ${evidence.httpStatus})` : ""
        }.`,
      );
      await loadDownloads(query);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setCheckingId(null);
    }
  }

  async function comparePrevious(downloadId: number) {
    setComparingId(downloadId);
    setComparison(null);
    setError(null);
    try {
      const result = await invoke<ComparisonResult>("compare_with_previous", { downloadId });
      setComparison(result);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setComparingId(null);
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
        <div className="phase-chip">Phase 3 · Living downloads</div>
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
        <div>
          <strong>{downloads.filter((item) => item.status === "CURRENT").length}</strong>
          <span>remote current</span>
        </div>
        <div>
          <strong>{downloads.filter((item) => item.status === "CHANGED").length}</strong>
          <span>remote changed</span>
        </div>
      </section>

      <form className="search" onSubmit={submitSearch}>
        <label htmlFor="search-input">Search provenance, versions and freshness evidence</label>
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
        <p className="privacy-note">
          Remote checks contact only the recorded HTTP(S) source when you press “Check remote source”.
          Local files are not uploaded.
        </p>
        {verificationNote ? <p className="verification-note">{verificationNote}</p> : null}
      </form>

      {error ? <p className="error">OriginKeep could not complete the requested evidence check: {error}</p> : null}

      {comparison ? (
        <section className="comparison-panel" aria-live="polite">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Local comparison · {comparison.kind}</p>
              <h2>
                {comparison.previousName} → {comparison.currentName}
              </h2>
            </div>
            <button type="button" className="secondary" onClick={() => setComparison(null)}>
              Close comparison
            </button>
          </div>
          <p className="comparison-summary">{comparison.summary}</p>
          <ul className="comparison-details">
            {comparison.details.map((detail, index) => (
              <li key={`${comparison.currentId}-${index}`}>{detail}</li>
            ))}
          </ul>
        </section>
      ) : null}

      {!loading && families.length > 0 ? (
        <section className="families" aria-labelledby="families-title">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Deterministic grouping</p>
              <h2 id="families-title">Version families</h2>
            </div>
            <p>
              Families use normalized initiating source identity; hashes decide exact content equality and
              HTTP validators provide remote freshness evidence.
            </p>
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
              Complete a browser download through the OriginKeep companion. OriginKeep will fingerprint the
              file, build deterministic version evidence locally, and let you explicitly check its recorded
              remote source.
            </p>
          </div>
        ) : null}

        {downloads.map((item) => {
          const evidence = evidenceByDownload.get(item.id);
          return (
            <article className="record" key={item.captureKey}>
              <div className="record-title">
                <div>
                  <h2>{item.fileName}</h2>
                  <p className="path">{item.localPath}</p>
                </div>
                <div className="badge-row">
                  {item.versionNumber !== null ? <span className="status">v{item.versionNumber}</span> : null}
                  <span className={`status remote-state ${item.status.toLowerCase()}`}>
                    {readableState(item.status)}
                  </span>
                  <span className={`status local-state ${item.localState.toLowerCase()}`}>
                    {readableState(item.localState)}
                  </span>
                </div>
              </div>

              <div className="record-actions">
                {canCheckRemote(item) ? (
                  <button
                    type="button"
                    className="secondary"
                    disabled={checkingId === item.id}
                    onClick={() => void checkRemote(item.id)}
                  >
                    {checkingId === item.id ? "Checking source…" : "Check remote source"}
                  </button>
                ) : null}
                {canCompare(item) ? (
                  <button
                    type="button"
                    className="secondary"
                    disabled={comparingId === item.id}
                    onClick={() => void comparePrevious(item.id)}
                  >
                    {comparingId === item.id ? "Comparing…" : `Compare with v${(item.versionNumber ?? 1) - 1}`}
                  </button>
                ) : null}
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
                <div>
                  <dt>Remote evidence</dt>
                  <dd>
                    {evidence
                      ? `${readableState(evidence.resultState)} · ${evidence.requestMethod} · ${
                          evidence.httpStatus === null ? "no HTTP status" : `HTTP ${evidence.httpStatus}`
                        } · ${evidence.checkedAt}`
                      : "Not checked yet"}
                  </dd>
                </div>
                <div>
                  <dt>Remote validator</dt>
                  <dd>
                    {evidence?.etag || evidence?.lastModified ||
                      (evidence?.contentLength !== null && evidence?.contentLength !== undefined
                        ? `Content-Length ${evidence.contentLength}`
                        : "No validator captured")}
                  </dd>
                </div>
                {evidence ? (
                  <div className="evidence-copy">
                    <dt>Why this state</dt>
                    <dd>{evidence.evidence}{evidence.error ? ` Error: ${evidence.error}` : ""}</dd>
                  </div>
                ) : null}
              </dl>
            </article>
          );
        })}
      </section>
    </main>
  );
}
