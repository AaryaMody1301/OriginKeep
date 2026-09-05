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

type LifecycleItem = {
  downloadId: number;
  fileName: string;
  originalPath: string;
  bytes: number | null;
  status: string;
  localState: string;
  sourceIdentity: string | null;
  versionNumber: number | null;
  duplicateOfId: number | null;
  lifecycleState: string;
  archivePath: string | null;
  reclaimable: boolean;
  archiveEligible: boolean;
  restoreEligible: boolean;
  recommendation: string;
  reason: string;
};

type LifecycleReview = {
  keepLatestVersions: number;
  includeDuplicates: boolean;
  summary: {
    trackedBytes: number;
    presentBytes: number;
    archivedBytes: number;
    reclaimableBytes: number;
    duplicateBytes: number;
    supersededBytes: number;
    protectedBytes: number;
    candidateCount: number;
    archivedCount: number;
    databaseHealth: string;
  };
  items: LifecycleItem[];
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
  const [remoteEvidence, setRemoteEvidence] = useState<RemoteEvidence[]>([]);
  const [review, setReview] = useState<LifecycleReview | null>(null);
  const [query, setQuery] = useState("");
  const [keepLatestVersions, setKeepLatestVersions] = useState(1);
  const [includeDuplicates, setIncludeDuplicates] = useState(true);
  const [loading, setLoading] = useState(true);
  const [verifying, setVerifying] = useState(false);
  const [checkingId, setCheckingId] = useState<number | null>(null);
  const [comparingId, setComparingId] = useState<number | null>(null);
  const [lifecycleBusyId, setLifecycleBusyId] = useState<number | null>(null);
  const [comparison, setComparison] = useState<ComparisonResult | null>(null);
  const [verificationNote, setVerificationNote] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadDownloads = useCallback(
    async (search = "", keepLatest = 1, duplicates = true) => {
      setLoading(true);
      setError(null);
      try {
        const lifecycle = await invoke<LifecycleReview>("lifecycle_review", {
          keepLatestVersions: keepLatest,
          includeDuplicates: duplicates,
        });
        const [rows, evidence] = await Promise.all([
          invoke<DownloadRecord[]>("list_downloads", { query: search.trim() || null }),
          invoke<RemoteEvidence[]>("list_remote_evidence"),
        ]);
        setReview(lifecycle);
        setDownloads(rows);
        setRemoteEvidence(evidence);
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    void loadDownloads("", 1, true);
  }, [loadDownloads]);

  const evidenceByDownload = useMemo(
    () => new Map(remoteEvidence.map((evidence) => [evidence.downloadId, evidence])),
    [remoteEvidence],
  );
  const lifecycleByDownload = useMemo(
    () => new Map((review?.items ?? []).map((item) => [item.downloadId, item])),
    [review],
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

  function displayedLocalState(item: DownloadRecord) {
    return lifecycleByDownload.get(item.id)?.lifecycleState === "ARCHIVED" ? "ARCHIVED" : item.localState;
  }

  function canCheckRemote(item: DownloadRecord) {
    return (
      Boolean(item.sourceIdentity) &&
      item.duplicateOfId === null &&
      item.status !== "SUPERSEDED" &&
      displayedLocalState(item) !== "ARCHIVED"
    );
  }

  function canCompare(item: DownloadRecord) {
    return (
      item.versionNumber !== null &&
      item.versionNumber > 1 &&
      item.duplicateOfId === null &&
      displayedLocalState(item) === "PRESENT"
    );
  }

  function submitSearch(event: FormEvent) {
    event.preventDefault();
    void loadDownloads(query, keepLatestVersions, includeDuplicates);
  }

  async function verifyLocal() {
    setVerifying(true);
    setVerificationNote(null);
    setError(null);
    try {
      const result = await invoke<VerificationSummary>("verify_local_files");
      setVerificationNote(
        `Verified ${result.checked} tracked paths: ${result.modified} modified, ${result.missing} missing, ${result.present} present${result.unavailable ? `, ${result.unavailable} unreadable` : ""}. Archived copies remain governed by the lifecycle ledger.`,
      );
      await loadDownloads(query, keepLatestVersions, includeDuplicates);
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
      await loadDownloads(query, keepLatestVersions, includeDuplicates);
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
      setComparison(await invoke<ComparisonResult>("compare_with_previous", { downloadId }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setComparingId(null);
    }
  }

  async function applyLifecycleAction(action: "archive_download" | "restore_download", downloadId: number) {
    setLifecycleBusyId(downloadId);
    setError(null);
    try {
      const result = await invoke<LifecycleItem>(action, { downloadId });
      setVerificationNote(
        `${result.fileName}: ${action === "archive_download" ? "archived after SHA-256 verification" : "restored to its original path"}.`,
      );
      await loadDownloads(query, keepLatestVersions, includeDuplicates);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLifecycleBusyId(null);
    }
  }

  async function applyRetentionPolicy(nextKeep = keepLatestVersions, nextDuplicates = includeDuplicates) {
    setKeepLatestVersions(nextKeep);
    setIncludeDuplicates(nextDuplicates);
    await loadDownloads(query, nextKeep, nextDuplicates);
  }

  const reviewItems = (review?.items ?? []).filter((item) => item.reclaimable || item.lifecycleState === "ARCHIVED");

  return (
    <main className="shell">
      <header className="hero">
        <div>
          <p className="eyebrow">Local-first download provenance</p>
          <h1>OriginKeep</h1>
          <p className="tagline">Downloads that remember where they came from.</p>
        </div>
        <div className="phase-chip">Phase 4 · Safe lifecycle</div>
      </header>

      <section className="summary" aria-label="Tracked download summary">
        <div><strong>{downloads.length}</strong><span>visible records</span></div>
        <div><strong>{families.length}</strong><span>version families</span></div>
        <div><strong>{downloads.filter((item) => item.duplicateOfId !== null).length}</strong><span>exact duplicates</span></div>
        <div><strong>{downloads.filter((item) => displayedLocalState(item) === "LOCAL_MODIFIED").length}</strong><span>locally modified</span></div>
        <div><strong>{review?.summary.archivedCount ?? 0}</strong><span>recoverably archived</span></div>
        <div><strong>{review?.summary.candidateCount ?? 0}</strong><span>cleanup candidates</span></div>
      </section>

      <form className="search" onSubmit={submitSearch}>
        <label htmlFor="search-input">Search provenance, versions, freshness and lifecycle evidence</label>
        <div className="search-row">
          <input
            id="search-input"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filename, source identity, URL, referrer, hash…"
          />
          <button type="submit">Search</button>
          <button type="button" className="secondary" onClick={() => void loadDownloads(query, keepLatestVersions, includeDuplicates)}>Refresh</button>
          <button type="button" className="secondary" disabled={verifying} onClick={() => void verifyLocal()}>
            {verifying ? "Verifying…" : "Verify local files"}
          </button>
        </div>
        <p className="privacy-note">
          Cleanup never silently deletes a tracked file. Archive copies stay local under OriginKeep application data and are verified against the recorded SHA-256 before the original is removed.
        </p>
        {verificationNote ? <p className="verification-note">{verificationNote}</p> : null}
      </form>

      {error ? <p className="error">OriginKeep could not complete the requested operation: {error}</p> : null}

      <section className="lifecycle-panel" aria-labelledby="downloads-review-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Deterministic cleanup preview</p>
            <h2 id="downloads-review-title">Downloads Review</h2>
          </div>
          <p>Database health: <strong>{review?.summary.databaseHealth ?? "Checking…"}</strong></p>
        </div>
        <div className="lifecycle-metrics">
          <div><strong>{formatBytes(review?.summary.trackedBytes ?? 0)}</strong><span>tracked</span></div>
          <div><strong>{formatBytes(review?.summary.reclaimableBytes ?? 0)}</strong><span>policy candidates</span></div>
          <div><strong>{formatBytes(review?.summary.archivedBytes ?? 0)}</strong><span>recoverable archive</span></div>
          <div><strong>{formatBytes(review?.summary.duplicateBytes ?? 0)}</strong><span>duplicate bytes</span></div>
          <div><strong>{formatBytes(review?.summary.supersededBytes ?? 0)}</strong><span>superseded bytes</span></div>
        </div>
        <div className="retention-controls">
          <label>
            Keep latest versions
            <select
              value={keepLatestVersions}
              onChange={(event) => void applyRetentionPolicy(Number(event.target.value), includeDuplicates)}
            >
              {[1, 2, 3, 5].map((value) => <option key={value} value={value}>{value}</option>)}
            </select>
          </label>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={includeDuplicates}
              onChange={(event) => void applyRetentionPolicy(keepLatestVersions, event.target.checked)}
            />
            Include exact duplicates
          </label>
        </div>
        <p className="privacy-note">
          The retention policy is preview-only. Every archive action remains explicit and reversible; locally modified or unhashed files are protected automatically.
        </p>
        {reviewItems.length === 0 ? <p className="empty">No cleanup candidates or archived files under this policy.</p> : null}
        <div className="lifecycle-list">
          {reviewItems.map((item) => (
            <article className="lifecycle-item" key={item.downloadId}>
              <div>
                <strong>{item.fileName}</strong>
                <p>{item.reason}</p>
                <small>{formatBytes(item.bytes)} · {readableState(item.lifecycleState)}</small>
              </div>
              {item.reclaimable ? (
                <button
                  type="button"
                  disabled={lifecycleBusyId === item.downloadId}
                  onClick={() => void applyLifecycleAction("archive_download", item.downloadId)}
                >
                  {lifecycleBusyId === item.downloadId ? "Archiving…" : "Archive safely"}
                </button>
              ) : null}
              {item.restoreEligible ? (
                <button
                  type="button"
                  className="secondary"
                  disabled={lifecycleBusyId === item.downloadId}
                  onClick={() => void applyLifecycleAction("restore_download", item.downloadId)}
                >
                  {lifecycleBusyId === item.downloadId ? "Restoring…" : "Restore"}
                </button>
              ) : null}
            </article>
          ))}
        </div>
      </section>

      {comparison ? (
        <section className="comparison-panel" aria-live="polite">
          <div className="section-heading">
            <div><p className="eyebrow">Local comparison · {comparison.kind}</p><h2>{comparison.previousName} → {comparison.currentName}</h2></div>
            <button type="button" className="secondary" onClick={() => setComparison(null)}>Close comparison</button>
          </div>
          <p className="comparison-summary">{comparison.summary}</p>
          <ul className="comparison-details">
            {comparison.details.map((detail, index) => <li key={`${comparison.currentId}-${index}`}>{detail}</li>)}
          </ul>
        </section>
      ) : null}

      {!loading && families.length > 0 ? (
        <section className="families" aria-labelledby="families-title">
          <div className="section-heading">
            <div><p className="eyebrow">Deterministic grouping</p><h2 id="families-title">Version families</h2></div>
            <p>Source identity establishes lineage, SHA-256 proves exact equality, and HTTP validators provide remote freshness evidence.</p>
          </div>
          {families.map((family) => {
            const versions = new Map<number, DownloadRecord[]>();
            for (const item of family.items) {
              if (item.versionNumber === null) continue;
              versions.set(item.versionNumber, [...(versions.get(item.versionNumber) ?? []), item]);
            }
            return (
              <article className="family-card" key={family.sourceIdentity}>
                <p className="family-source">{family.sourceIdentity}</p>
                <div className="timeline">
                  {Array.from(versions.entries()).sort(([left], [right]) => right - left).map(([version, items]) => {
                    const primary = items.find((item) => item.duplicateOfId === null) ?? items[0];
                    const duplicates = items.filter((item) => item.duplicateOfId !== null).length;
                    return (
                      <div className="version-node" key={version}>
                        <span className="version-number">v{version}</span>
                        <div><strong>{primary.fileName}</strong><p>{shortHash(primary.sha256)}</p><p>{readableState(primary.status)} · {readableState(displayedLocalState(primary))}{duplicates ? ` · ${duplicates} duplicate${duplicates === 1 ? "" : "s"}` : ""}</p></div>
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
          <div className="empty-card"><h2>No tracked downloads yet</h2><p>Complete a browser download through the OriginKeep companion to begin building local provenance and lifecycle evidence.</p></div>
        ) : null}
        {downloads.map((item) => {
          const evidence = evidenceByDownload.get(item.id);
          const lifecycle = lifecycleByDownload.get(item.id);
          const localState = displayedLocalState(item);
          return (
            <article className="record" key={item.captureKey}>
              <div className="record-title">
                <div><h2>{item.fileName}</h2><p className="path">{item.localPath}</p></div>
                <div className="badge-row">
                  {item.versionNumber !== null ? <span className="status">v{item.versionNumber}</span> : null}
                  <span className={`status remote-state ${item.status.toLowerCase()}`}>{readableState(item.status)}</span>
                  <span className={`status local-state ${localState.toLowerCase()}`}>{readableState(localState)}</span>
                </div>
              </div>
              <div className="record-actions">
                {canCheckRemote(item) ? <button type="button" className="secondary" disabled={checkingId === item.id} onClick={() => void checkRemote(item.id)}>{checkingId === item.id ? "Checking source…" : "Check remote source"}</button> : null}
                {canCompare(item) ? <button type="button" className="secondary" disabled={comparingId === item.id} onClick={() => void comparePrevious(item.id)}>{comparingId === item.id ? "Comparing…" : `Compare with v${(item.versionNumber ?? 1) - 1}`}</button> : null}
                {lifecycle?.reclaimable ? <button type="button" disabled={lifecycleBusyId === item.id} onClick={() => void applyLifecycleAction("archive_download", item.id)}>{lifecycleBusyId === item.id ? "Archiving…" : "Archive safely"}</button> : null}
                {lifecycle?.restoreEligible ? <button type="button" className="secondary" disabled={lifecycleBusyId === item.id} onClick={() => void applyLifecycleAction("restore_download", item.id)}>{lifecycleBusyId === item.id ? "Restoring…" : "Restore archived copy"}</button> : null}
              </div>
              <dl>
                <div><dt>Source identity</dt><dd>{item.sourceIdentity || "No canonical HTTP(S) identity"}</dd></div>
                <div><dt>Origin</dt><dd>{item.originalUrl}</dd></div>
                <div><dt>Final URL</dt><dd>{item.finalUrl || "Not reported"}</dd></div>
                <div><dt>Exact duplicate</dt><dd>{item.duplicateOfId === null ? "No" : `Matches record #${item.duplicateOfId}`}</dd></div>
                <div><dt>Integrity</dt><dd>{shortHash(item.sha256)}</dd></div>
                <div><dt>Size / MIME</dt><dd>{formatBytes(item.bytes)} · {item.mimeType || "Unknown MIME"}</dd></div>
                <div><dt>Lifecycle</dt><dd>{lifecycle ? `${readableState(lifecycle.lifecycleState)} · ${lifecycle.reason}` : "Active"}</dd></div>
                {lifecycle?.archivePath ? <div><dt>Archive path</dt><dd>{lifecycle.archivePath}</dd></div> : null}
                <div><dt>Remote evidence</dt><dd>{evidence ? `${readableState(evidence.resultState)} · ${evidence.requestMethod} · ${evidence.httpStatus === null ? "no HTTP status" : `HTTP ${evidence.httpStatus}`} · ${evidence.checkedAt}` : "Not checked yet"}</dd></div>
                <div><dt>Remote validator</dt><dd>{evidence?.etag || evidence?.lastModified || (evidence?.contentLength !== null && evidence?.contentLength !== undefined ? `Content-Length ${evidence.contentLength}` : "No validator captured")}</dd></div>
                {evidence ? <div className="evidence-copy"><dt>Why this state</dt><dd>{evidence.evidence}{evidence.error ? ` Error: ${evidence.error}` : ""}</dd></div> : null}
              </dl>
            </article>
          );
        })}
      </section>
    </main>
  );
}
