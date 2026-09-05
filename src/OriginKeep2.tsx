import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./passport.css";

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

type PassportSummary = {
  downloadId: number;
  purpose: string;
  expiresAt: string | null;
  pageTitle: string | null;
  pageUrl: string | null;
  locationCount: number;
  trustSignalCount: number;
};

type TrustObservation = {
  kind: string;
  state: string;
  summary: string;
  details: string | null;
  checkedAt: string;
};

type FileLocation = {
  path: string;
  state: string;
  firstSeenAt: string;
  lastSeenAt: string;
};

type FilePassport = {
  downloadId: number;
  fileName: string;
  localPath: string;
  mimeType: string | null;
  bytes: number | null;
  sha256: string | null;
  originalUrl: string;
  finalUrl: string | null;
  referrer: string | null;
  sourceIdentity: string | null;
  downloadedAt: string | null;
  versionNumber: number | null;
  duplicateOfId: number | null;
  status: string;
  localState: string;
  lifecycleState: string;
  archivePath: string | null;
  browserName: string | null;
  pageTitle: string | null;
  pageUrl: string | null;
  linkText: string | null;
  contextText: string | null;
  contextSource: string | null;
  purpose: string;
  note: string | null;
  expiresAt: string | null;
  sigstoreIdentity: string | null;
  sigstoreIssuer: string | null;
  remoteState: string | null;
  remoteEvidence: string | null;
  remoteCheckedAt: string | null;
  locations: FileLocation[];
  trust: TrustObservation[];
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

type GraphNode = { id: string; kind: string; label: string; detail: string | null };
type GraphEdge = { from: string; to: string; relation: string };
type OriginGraph = { nodes: GraphNode[]; edges: GraphEdge[] };

type BrowserSetupResult = {
  platform: string;
  hostPath: string;
  manifestsWritten: string[];
  note: string;
};

type VerificationSummary = {
  checked: number;
  present: number;
  modified: number;
  missing: number;
  unavailable: number;
};

const PURPOSES = [
  "UNSPECIFIED",
  "REFERENCE",
  "READ_LATER",
  "TEMPORARY",
  "WORK",
  "RECEIPT",
  "INSTALLER",
  "DATASET",
  "OTHER",
];

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

function readable(value: string | null | undefined) {
  return value ? value.replaceAll("_", " ") : "Unknown";
}

function shortHash(hash: string | null) {
  return hash ? `${hash.slice(0, 14)}…` : "No fingerprint";
}

function emptyToNull(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

export default function OriginKeep2() {
  const [downloads, setDownloads] = useState<DownloadRecord[]>([]);
  const [summaries, setSummaries] = useState<PassportSummary[]>([]);
  const [review, setReview] = useState<LifecycleReview | null>(null);
  const [graph, setGraph] = useState<OriginGraph>({ nodes: [], edges: [] });
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [passport, setPassport] = useState<FilePassport | null>(null);
  const [comparison, setComparison] = useState<ComparisonResult | null>(null);
  const [keepLatest, setKeepLatest] = useState(1);
  const [includeDuplicates, setIncludeDuplicates] = useState(true);
  const [purpose, setPurpose] = useState("UNSPECIFIED");
  const [note, setNote] = useState("");
  const [expiresAt, setExpiresAt] = useState("");
  const [sigstoreIdentity, setSigstoreIdentity] = useState("");
  const [sigstoreIssuer, setSigstoreIssuer] = useState("");
  const [reconnectPath, setReconnectPath] = useState("");
  const [importPath, setImportPath] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadAll = useCallback(
    async (search = query, latest = keepLatest, duplicates = includeDuplicates) => {
      setBusy("load");
      setError(null);
      try {
        const [rows, passportRows, lifecycle, nextGraph] = await Promise.all([
          invoke<DownloadRecord[]>("list_downloads", { query: search.trim() || null }),
          invoke<PassportSummary[]>("list_passport_summaries"),
          invoke<LifecycleReview>("lifecycle_review", {
            keepLatestVersions: latest,
            includeDuplicates: duplicates,
          }),
          invoke<OriginGraph>("origin_graph"),
        ]);
        setDownloads(rows);
        setSummaries(passportRows);
        setReview(lifecycle);
        setGraph(nextGraph);
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        setBusy(null);
      }
    },
    [query, keepLatest, includeDuplicates],
  );

  useEffect(() => {
    void loadAll("", 1, true);
  }, []); // Initial database snapshot only; subsequent changes use explicit refreshes.

  const summaryById = useMemo(
    () => new Map(summaries.map((item) => [item.downloadId, item])),
    [summaries],
  );
  const lifecycleById = useMemo(
    () => new Map((review?.items ?? []).map((item) => [item.downloadId, item])),
    [review],
  );
  const graphNodeById = useMemo(
    () => new Map(graph.nodes.map((node) => [node.id, node])),
    [graph.nodes],
  );

  async function openPassport(downloadId: number) {
    setSelectedId(downloadId);
    setBusy(`passport:${downloadId}`);
    setError(null);
    setComparison(null);
    try {
      const next = await invoke<FilePassport>("get_file_passport", { downloadId });
      setPassport(next);
      setPurpose(next.purpose);
      setNote(next.note ?? "");
      setExpiresAt(next.expiresAt ?? "");
      setSigstoreIdentity(next.sigstoreIdentity ?? "");
      setSigstoreIssuer(next.sigstoreIssuer ?? "");
      setReconnectPath("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function refreshSelected() {
    if (selectedId === null) return;
    await openPassport(selectedId);
  }

  async function savePassport(event: FormEvent) {
    event.preventDefault();
    if (selectedId === null) return;
    setBusy("save-passport");
    setError(null);
    try {
      const next = await invoke<FilePassport>("update_passport_metadata", {
        downloadId: selectedId,
        purpose,
        note: emptyToNull(note),
        expiresAt: emptyToNull(expiresAt),
        sigstoreIdentity: emptyToNull(sigstoreIdentity),
        sigstoreIssuer: emptyToNull(sigstoreIssuer),
      });
      setPassport(next);
      setNotice("File Passport metadata saved locally.");
      await loadAll();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function verifyLocal() {
    setBusy("verify-local");
    setError(null);
    try {
      const result = await invoke<VerificationSummary>("verify_local_files");
      setNotice(
        `Verified ${result.checked} paths: ${result.present} present, ${result.modified} modified, ${result.missing} missing${result.unavailable ? `, ${result.unavailable} unreadable` : ""}.`,
      );
      await invoke("refresh_locations");
      await loadAll();
      await refreshSelected();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function refreshTrust() {
    if (selectedId === null) return;
    setBusy("trust");
    setError(null);
    try {
      const observations = await invoke<TrustObservation[]>("refresh_trust", {
        downloadId: selectedId,
      });
      setPassport((current) => (current ? { ...current, trust: observations } : current));
      setNotice("Trust Lens refreshed using local evidence and available verifier tools.");
      await loadAll();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function exportPassport() {
    if (selectedId === null) return;
    setBusy("export");
    setError(null);
    try {
      const result = await invoke<{ sidecarPath: string }>("export_passport", {
        downloadId: selectedId,
      });
      setNotice(`Portable passport written to ${result.sidecarPath}`);
      await refreshSelected();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function importPortablePassport(event: FormEvent) {
    event.preventDefault();
    if (!importPath.trim()) return;
    setBusy("import");
    setError(null);
    try {
      const imported = await invoke<FilePassport>("import_passport", {
        sidecarPath: importPath.trim(),
      });
      setNotice(`Imported and SHA-256 verified passport for ${imported.fileName}.`);
      setImportPath("");
      await loadAll();
      await openPassport(imported.downloadId);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function reconnect(event: FormEvent) {
    event.preventDefault();
    if (selectedId === null || !reconnectPath.trim()) return;
    setBusy("reconnect");
    setError(null);
    try {
      const result = await invoke<{ primaryPathUpdated: boolean; locationCount: number }>(
        "reconnect_file",
        { downloadId: selectedId, newPath: reconnectPath.trim() },
      );
      setNotice(
        `Exact content identity verified. ${result.locationCount} known location(s)${result.primaryPathUpdated ? "; primary path repaired" : ""}.`,
      );
      setReconnectPath("");
      await loadAll();
      await refreshSelected();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function checkRemote() {
    if (selectedId === null) return;
    setBusy("remote");
    setError(null);
    try {
      const evidence = await invoke<RemoteEvidence>("check_remote_freshness", {
        downloadId: selectedId,
      });
      setNotice(`${readable(evidence.resultState)}: ${evidence.evidence}`);
      await loadAll();
      await refreshSelected();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function comparePrevious() {
    if (selectedId === null) return;
    setBusy("compare");
    setError(null);
    try {
      setComparison(
        await invoke<ComparisonResult>("compare_with_previous", { downloadId: selectedId }),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function lifecycle(action: "archive_download" | "restore_download") {
    if (selectedId === null) return;
    setBusy(action);
    setError(null);
    try {
      const result = await invoke<LifecycleItem>(action, { downloadId: selectedId });
      setNotice(
        action === "archive_download"
          ? `${result.fileName} archived after integrity verification.`
          : `${result.fileName} restored without overwriting conflicting bytes.`,
      );
      await loadAll();
      await refreshSelected();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function installBrowserIntegration() {
    setBusy("browser-setup");
    setError(null);
    try {
      const result = await invoke<BrowserSetupResult>("install_browser_integration");
      setNotice(`${readable(result.platform)}: ${result.note}`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  function submitSearch(event: FormEvent) {
    event.preventDefault();
    void loadAll(query, keepLatest, includeDuplicates);
  }

  const selectedLifecycle = selectedId === null ? null : lifecycleById.get(selectedId) ?? null;
  const reviewItems = (review?.items ?? []).filter(
    (item) => item.reclaimable || item.lifecycleState === "ARCHIVED",
  );
  const graphEdges = graph.edges.slice(0, 80);

  return (
    <main className="v2-shell">
      <header className="v2-hero">
        <div>
          <p className="v2-eyebrow">OriginKeep 2.0 · Universal File Passport</p>
          <h1>Every file remembers.</h1>
          <p>
            Where it came from, why you saved it, whether it changed, who can verify it, and how to get it back.
          </p>
        </div>
        <div className="v2-hero-actions">
          <button type="button" onClick={() => void installBrowserIntegration()} disabled={busy !== null}>
            Browser integration
          </button>
          <button type="button" className="v2-secondary" onClick={() => void verifyLocal()} disabled={busy !== null}>
            Verify local files
          </button>
        </div>
      </header>

      <section className="v2-metrics">
        <div><strong>{downloads.length}</strong><span>tracked files</span></div>
        <div><strong>{summaries.reduce((total, item) => total + item.locationCount, 0)}</strong><span>known content locations</span></div>
        <div><strong>{summaries.reduce((total, item) => total + item.trustSignalCount, 0)}</strong><span>trust observations</span></div>
        <div><strong>{review?.summary.candidateCount ?? 0}</strong><span>review candidates</span></div>
        <div><strong>{formatBytes(review?.summary.reclaimableBytes ?? 0)}</strong><span>recoverable space</span></div>
      </section>

      <section className="v2-toolbar">
        <form onSubmit={submitSearch} className="v2-search">
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search file, source, URL, hash…" />
          <button type="submit" disabled={busy === "load"}>Search</button>
          <button type="button" className="v2-secondary" onClick={() => void loadAll()} disabled={busy === "load"}>Refresh</button>
        </form>
        <form onSubmit={importPortablePassport} className="v2-import">
          <input value={importPath} onChange={(event) => setImportPath(event.target.value)} placeholder="Path to file.originkeep.json" />
          <button type="submit" className="v2-secondary" disabled={busy === "import"}>Import passport</button>
        </form>
      </section>

      {notice ? <p className="v2-notice">{notice}</p> : null}
      {error ? <p className="v2-error">{error}</p> : null}

      <div className="v2-layout">
        <section className="v2-library" aria-label="File passport library">
          <div className="v2-section-heading">
            <div><p className="v2-eyebrow">Passport library</p><h2>Files</h2></div>
            <span>{busy === "load" ? "Refreshing…" : `${downloads.length} visible`}</span>
          </div>
          {downloads.length === 0 ? <p className="v2-empty">No tracked downloads match this view.</p> : null}
          <div className="v2-file-list">
            {downloads.map((item) => {
              const summary = summaryById.get(item.id);
              const lifecycleItem = lifecycleById.get(item.id);
              return (
                <button
                  type="button"
                  className={`v2-file-card${selectedId === item.id ? " selected" : ""}`}
                  key={item.id}
                  onClick={() => void openPassport(item.id)}
                >
                  <div className="v2-file-title">
                    <strong>{item.fileName}</strong>
                    <span>{item.versionNumber ? `v${item.versionNumber}` : "unversioned"}</span>
                  </div>
                  <p>{summary?.pageTitle || item.sourceIdentity || item.originalUrl}</p>
                  <div className="v2-badges">
                    <span>{readable(summary?.purpose || "UNSPECIFIED")}</span>
                    <span>{readable(lifecycleItem?.lifecycleState || item.localState)}</span>
                    <span>{summary?.locationCount ?? 0} location(s)</span>
                    {item.duplicateOfId ? <span>exact duplicate</span> : null}
                  </div>
                  <small>{shortHash(item.sha256)} · {formatBytes(item.bytes)}</small>
                </button>
              );
            })}
          </div>
        </section>

        <section className="v2-passport" aria-label="Selected file passport">
          {!passport ? (
            <div className="v2-empty-card">
              <p className="v2-eyebrow">File Passport</p>
              <h2>Select a file</h2>
              <p>Origin, context, identity, integrity, freshness, authenticity, lineage and recovery evidence appear here.</p>
            </div>
          ) : (
            <>
              <div className="v2-passport-title">
                <div>
                  <p className="v2-eyebrow">File Passport #{passport.downloadId}</p>
                  <h2>{passport.fileName}</h2>
                  <p>{passport.pageTitle || passport.sourceIdentity || passport.originalUrl}</p>
                </div>
                <button type="button" className="v2-secondary" onClick={() => void exportPassport()} disabled={busy === "export"}>
                  Export passport
                </button>
              </div>

              <div className="v2-passport-grid">
                <article><h3>Origin</h3><dl><dt>Original URL</dt><dd>{passport.originalUrl}</dd><dt>Referrer</dt><dd>{passport.referrer || "Unavailable"}</dd><dt>Browser</dt><dd>{passport.browserName || "Legacy capture"}</dd><dt>Downloaded</dt><dd>{passport.downloadedAt || "Unknown"}</dd></dl></article>
                <article><h3>Context</h3><dl><dt>Page</dt><dd>{passport.pageTitle || "Not captured"}</dd><dt>Page URL</dt><dd>{passport.pageUrl || "Not captured"}</dd><dt>Clicked text</dt><dd>{passport.linkText || "Not captured"}</dd><dt>Nearby context</dt><dd>{passport.contextText || "Not captured"}</dd></dl></article>
                <article><h3>Identity</h3><dl><dt>SHA-256</dt><dd className="v2-mono">{passport.sha256 || "No baseline"}</dd><dt>Source family</dt><dd>{passport.sourceIdentity || "Unknown"}</dd><dt>Version</dt><dd>{passport.versionNumber ?? "Unknown"}</dd><dt>Duplicate of</dt><dd>{passport.duplicateOfId ?? "Primary"}</dd></dl></article>
                <article><h3>Freshness</h3><dl><dt>Remote state</dt><dd>{readable(passport.remoteState)}</dd><dt>Checked</dt><dd>{passport.remoteCheckedAt || "Never"}</dd><dt>Evidence</dt><dd>{passport.remoteEvidence || "No remote check yet"}</dd></dl></article>
                <article><h3>Recovery</h3><dl><dt>Local state</dt><dd>{readable(passport.localState)}</dd><dt>Lifecycle</dt><dd>{readable(passport.lifecycleState)}</dd><dt>Primary path</dt><dd>{passport.localPath}</dd><dt>Archive path</dt><dd>{passport.archivePath || "Not archived"}</dd></dl></article>
                <article><h3>Purpose</h3><dl><dt>Purpose</dt><dd>{readable(passport.purpose)}</dd><dt>Review/expiry</dt><dd>{passport.expiresAt || "Not set"}</dd><dt>Note</dt><dd>{passport.note || "No note"}</dd></dl></article>
              </div>

              <article className="v2-panel">
                <div className="v2-section-heading"><div><p className="v2-eyebrow">Content identity</p><h3>Known locations</h3></div><span>{passport.locations.length}</span></div>
                {passport.locations.length === 0 ? <p>No verified locations yet.</p> : null}
                {passport.locations.map((location) => (
                  <div className="v2-evidence-row" key={location.path}><strong>{readable(location.state)}</strong><span>{location.path}</span></div>
                ))}
                <form onSubmit={reconnect} className="v2-inline-form">
                  <input value={reconnectPath} onChange={(event) => setReconnectPath(event.target.value)} placeholder="Moved/copied file path" />
                  <button type="submit" className="v2-secondary" disabled={busy === "reconnect"}>Verify & reconnect</button>
                </form>
                <small>OriginKeep reconnects only after an exact SHA-256 match. Similar names never count as identity.</small>
              </article>

              <article className="v2-panel">
                <div className="v2-section-heading"><div><p className="v2-eyebrow">Evidence, not a score</p><h3>Trust Lens</h3></div><button type="button" className="v2-secondary" onClick={() => void refreshTrust()} disabled={busy === "trust"}>Refresh evidence</button></div>
                {passport.trust.length === 0 ? <p>No trust checks recorded yet.</p> : null}
                {passport.trust.map((item) => (
                  <div className="v2-trust-row" key={item.kind}><div><strong>{readable(item.kind)}</strong><span className={`v2-state state-${item.state.toLowerCase()}`}>{readable(item.state)}</span></div><p>{item.summary}</p>{item.details ? <small>{item.details}</small> : null}</div>
                ))}
              </article>

              <form className="v2-panel v2-edit" onSubmit={savePassport}>
                <div className="v2-section-heading"><div><p className="v2-eyebrow">Your local memory</p><h3>Purpose, note & verifier policy</h3></div><button type="submit" disabled={busy === "save-passport"}>Save</button></div>
                <label>Purpose<select value={purpose} onChange={(event) => setPurpose(event.target.value)}>{PURPOSES.map((item) => <option value={item} key={item}>{readable(item)}</option>)}</select></label>
                <label>Review / expiry<input value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} placeholder="2026-12-31 or your own reminder text" /></label>
                <label className="v2-wide">Note<textarea value={note} onChange={(event) => setNote(event.target.value)} placeholder="Why did I save this?" rows={3} /></label>
                <label>Expected Sigstore identity<input value={sigstoreIdentity} onChange={(event) => setSigstoreIdentity(event.target.value)} placeholder="publisher@example.com or workflow identity" /></label>
                <label>Expected Sigstore OIDC issuer<input value={sigstoreIssuer} onChange={(event) => setSigstoreIssuer(event.target.value)} placeholder="https://token.actions.githubusercontent.com" /></label>
              </form>

              <div className="v2-actions">
                {passport.sourceIdentity && passport.duplicateOfId === null ? <button type="button" onClick={() => void checkRemote()} disabled={busy === "remote"}>Check remote source</button> : null}
                {passport.versionNumber && passport.versionNumber > 1 && passport.duplicateOfId === null ? <button type="button" className="v2-secondary" onClick={() => void comparePrevious()} disabled={busy === "compare"}>Compare previous version</button> : null}
                {selectedLifecycle?.reclaimable ? <button type="button" className="v2-secondary" onClick={() => void lifecycle("archive_download")} disabled={busy === "archive_download"}>Archive safely</button> : null}
                {selectedLifecycle?.restoreEligible ? <button type="button" className="v2-secondary" onClick={() => void lifecycle("restore_download")} disabled={busy === "restore_download"}>Restore</button> : null}
              </div>

              {comparison ? <article className="v2-panel"><div className="v2-section-heading"><div><p className="v2-eyebrow">{comparison.kind} comparison</p><h3>{comparison.previousName} → {comparison.currentName}</h3></div><button type="button" className="v2-secondary" onClick={() => setComparison(null)}>Close</button></div><p>{comparison.summary}</p><ul>{comparison.details.map((detail, index) => <li key={`${index}-${detail}`}>{detail}</li>)}</ul></article> : null}
            </>
          )}
        </section>
      </div>

      <section className="v2-review">
        <div className="v2-section-heading">
          <div><p className="v2-eyebrow">Recoverable lifecycle</p><h2>Downloads Review</h2></div>
          <div className="v2-review-controls"><label>Keep latest<select value={keepLatest} onChange={(event) => { const value = Number(event.target.value); setKeepLatest(value); void loadAll(query, value, includeDuplicates); }}>{[1, 2, 3, 5].map((item) => <option value={item} key={item}>{item}</option>)}</select></label><label><input type="checkbox" checked={includeDuplicates} onChange={(event) => { setIncludeDuplicates(event.target.checked); void loadAll(query, keepLatest, event.target.checked); }} /> Include exact duplicates</label></div>
        </div>
        <div className="v2-review-grid">
          {reviewItems.slice(0, 30).map((item) => <article key={item.downloadId}><strong>{item.fileName}</strong><p>{item.reason}</p><small>{formatBytes(item.bytes)} · {readable(item.lifecycleState)}</small><button type="button" className="v2-secondary" onClick={() => void openPassport(item.downloadId)}>Review passport</button></article>)}
          {reviewItems.length === 0 ? <p className="v2-empty">Nothing needs cleanup under the current policy.</p> : null}
        </div>
      </section>

      <section className="v2-graph">
        <div className="v2-section-heading"><div><p className="v2-eyebrow">Origin Graph</p><h2>Source → version → content → location</h2></div><span>{graph.nodes.length} nodes · {graph.edges.length} edges</span></div>
        <div className="v2-graph-list">
          {graphEdges.map((edge, index) => {
            const from = graphNodeById.get(edge.from);
            const to = graphNodeById.get(edge.to);
            return <div key={`${edge.from}-${edge.to}-${index}`}><span>{from?.label || edge.from}</span><strong>{readable(edge.relation)}</strong><span>{to?.label || edge.to}</span></div>;
          })}
          {graph.edges.length > graphEdges.length ? <small>Showing the first {graphEdges.length} deterministic edges.</small> : null}
        </div>
      </section>
    </main>
  );
}
