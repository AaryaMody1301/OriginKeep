import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type FileLocation = {
  path: string;
  firstSeen: string;
  lastSeen: string;
  isCurrent: boolean;
};

type FilePassport = {
  downloadId: number;
  fileName: string;
  localPath: string;
  sha256: string | null;
  bytes: number | null;
  originalUrl: string;
  finalUrl: string | null;
  referrer: string | null;
  sourceIdentity: string | null;
  status: string;
  localState: string;
  versionNumber: number | null;
  duplicateOfId: number | null;
  pageTitle: string | null;
  pageUrl: string | null;
  linkText: string | null;
  contextText: string | null;
  browserName: string | null;
  userNote: string | null;
  purpose: string | null;
  expiresAt: string | null;
  retentionAction: string;
  locations: FileLocation[];
  osProvenance: string | null;
};

type TrustEvidence = {
  kind: string;
  state: string;
  summary: string;
  details: string[];
};

type TrustReport = {
  downloadId: number;
  fileName: string;
  evidence: TrustEvidence[];
};

type OriginGraph = {
  nodes: { id: string; kind: string; label: string; state: string | null }[];
  edges: { from: string; to: string; relation: string }[];
};

type MoveScanResult = {
  scannedFiles: number;
  matchedFiles: number;
  truncated: boolean;
};

const purposes = ["Reference", "Read later", "Temporary", "Work", "Receipt", "Installer", "Dataset", "Other"];
const retentionActions = [
  ["REVIEW", "Review manually"],
  ["NEVER_ARCHIVE", "Never archive"],
  ["ARCHIVE_WHEN_SUPERSEDED", "Archive when superseded (intent)"],
  ["ARCHIVE_AFTER_EXPIRY", "Archive after expiry (intent)"],
];

function shortHash(value: string | null) {
  return value ? `${value.slice(0, 14)}…` : "No fingerprint";
}

function readable(value: string) {
  return value.replaceAll("_", " ");
}

export default function PassportCenter() {
  const [passports, setPassports] = useState<FilePassport[]>([]);
  const [graph, setGraph] = useState<OriginGraph>({ nodes: [], edges: [] });
  const [trust, setTrust] = useState<TrustReport | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [passportPath, setPassportPath] = useState("");
  const [scanRoot, setScanRoot] = useState("");
  const [relink, setRelink] = useState<Record<number, string>>({});
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [items, originGraph] = await Promise.all([
        invoke<FilePassport[]>("list_passports"),
        invoke<OriginGraph>("origin_graph"),
      ]);
      setPassports(items);
      setGraph(originGraph);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const sourceCount = useMemo(
    () => graph.nodes.filter((node) => node.kind === "SOURCE").length,
    [graph],
  );

  async function runFor<T>(downloadId: number, command: string, args: Record<string, unknown> = {}) {
    setBusyId(downloadId);
    setError(null);
    try {
      const result = await invoke<T>(command, { downloadId, ...args });
      await load();
      return result;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    } finally {
      setBusyId(null);
    }
  }

  async function save(passport: FilePassport) {
    const result = await runFor<FilePassport>(passport.downloadId, "update_passport_metadata", {
      userNote: passport.userNote,
      purpose: passport.purpose,
      expiresAt: passport.expiresAt,
      retentionAction: passport.retentionAction,
    });
    if (result) setNotice(`${passport.fileName}: File Passport intent saved.`);
  }

  async function inspectTrust(downloadId: number) {
    const report = await runFor<TrustReport>(downloadId, "inspect_trust");
    if (report) setTrust(report);
  }

  async function exportPassport(passport: FilePassport) {
    const result = await runFor<{ path: string }>(passport.downloadId, "export_passport");
    if (result) setNotice(`Portable passport written beside the file: ${result.path}`);
  }

  async function importPassport() {
    if (!passportPath.trim()) return;
    setError(null);
    try {
      const imported = await invoke<FilePassport>("import_passport", { passportPath: passportPath.trim() });
      setPassportPath("");
      setNotice(`${imported.fileName}: imported only after SHA-256 verification.`);
      await load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function scanMoves() {
    if (!scanRoot.trim()) return;
    setError(null);
    try {
      const result = await invoke<MoveScanResult>("scan_for_moves", { root: scanRoot.trim(), maxFiles: 20000 });
      setNotice(
        `Move scan inspected ${result.scannedFiles} files and reconnected ${result.matchedFiles} exact SHA-256 match${result.matchedFiles === 1 ? "" : "es"}${result.truncated ? "; scan limit reached" : ""}.`,
      );
      await load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  function patchPassport(downloadId: number, patch: Partial<FilePassport>) {
    setPassports((items) => items.map((item) => (item.downloadId === downloadId ? { ...item, ...patch } : item)));
  }

  return (
    <section className="passport-center" aria-labelledby="passport-center-title">
      <div className="passport-hero">
        <div>
          <p className="eyebrow">OriginKeep 2.0 · Universal file memory</p>
          <h1 id="passport-center-title">File Passports</h1>
          <p>Every tracked file can carry origin, context, identity, integrity, authenticity, freshness, lineage and recovery evidence.</p>
        </div>
        <div className="passport-stats">
          <strong>{passports.length}</strong><span>passports</span>
          <strong>{sourceCount}</strong><span>origins</span>
        </div>
      </div>

      <div className="passport-tools">
        <label>
          Import portable passport
          <input value={passportPath} onChange={(event) => setPassportPath(event.target.value)} placeholder="/path/report.pdf.originkeep.json" />
        </label>
        <button type="button" onClick={() => void importPassport()}>Import + verify</button>
        <label>
          Find moved tracked files
          <input value={scanRoot} onChange={(event) => setScanRoot(event.target.value)} placeholder="Folder to scan" />
        </label>
        <button type="button" className="secondary" onClick={() => void scanMoves()}>Scan exact identities</button>
      </div>

      <p className="privacy-note">
        Portable passports exclude absolute local paths. Imports, relinks and move recovery must match the recorded SHA-256 before OriginKeep reconnects identity.
      </p>
      {notice ? <p className="verification-note">{notice}</p> : null}
      {error ? <p className="error">OriginKeep Passport: {error}</p> : null}

      <div className="passport-grid">
        {passports.map((passport) => (
          <article className="passport-card" key={passport.downloadId}>
            <header>
              <div>
                <p className="eyebrow">Passport #{passport.downloadId}{passport.versionNumber ? ` · v${passport.versionNumber}` : ""}</p>
                <h2>{passport.fileName}</h2>
                <p className="path">{passport.localPath}</p>
              </div>
              <div className="badge-row">
                <span className="status">{readable(passport.status)}</span>
                <span className="status">{readable(passport.localState)}</span>
              </div>
            </header>

            <div className="passport-evidence-grid">
              <div><strong>Origin</strong><span>{passport.sourceIdentity || passport.originalUrl}</span></div>
              <div><strong>Context</strong><span>{passport.pageTitle || passport.pageUrl || "No matched page context"}</span></div>
              <div><strong>Identity</strong><span>{shortHash(passport.sha256)}</span></div>
              <div><strong>Browser</strong><span>{passport.browserName || "Unknown / imported"}</span></div>
              <div><strong>Lineage</strong><span>{passport.duplicateOfId ? `Exact duplicate of #${passport.duplicateOfId}` : passport.versionNumber ? `Version ${passport.versionNumber}` : "No source family"}</span></div>
              <div><strong>OS provenance</strong><span>{passport.osProvenance ? "Imported" : "Not imported"}</span></div>
            </div>

            {passport.linkText || passport.contextText ? (
              <details>
                <summary>Why this was downloaded</summary>
                {passport.linkText ? <p><strong>Clicked:</strong> {passport.linkText}</p> : null}
                {passport.contextText ? <p>{passport.contextText}</p> : null}
                {passport.pageUrl ? <p className="path">{passport.pageUrl}</p> : null}
              </details>
            ) : null}

            <div className="passport-intent">
              <label>Purpose
                <select value={passport.purpose || ""} onChange={(event) => patchPassport(passport.downloadId, { purpose: event.target.value || null })}>
                  <option value="">Unspecified</option>
                  {purposes.map((purpose) => <option key={purpose}>{purpose}</option>)}
                </select>
              </label>
              <label>Review / expiry
                <input type="date" value={passport.expiresAt?.slice(0, 10) || ""} onChange={(event) => patchPassport(passport.downloadId, { expiresAt: event.target.value || null })} />
              </label>
              <label>Retention intent
                <select value={passport.retentionAction} onChange={(event) => patchPassport(passport.downloadId, { retentionAction: event.target.value })}>
                  {retentionActions.map(([value, label]) => <option value={value} key={value}>{label}</option>)}
                </select>
              </label>
              <label className="passport-note">Note
                <textarea value={passport.userNote || ""} onChange={(event) => patchPassport(passport.downloadId, { userNote: event.target.value || null })} placeholder="Why did you save this?" />
              </label>
            </div>

            <div className="record-actions passport-actions">
              <button type="button" disabled={busyId === passport.downloadId} onClick={() => void save(passport)}>Save intent</button>
              <button type="button" className="secondary" disabled={busyId === passport.downloadId} onClick={() => void inspectTrust(passport.downloadId)}>Trust Lens</button>
              <button type="button" className="secondary" disabled={busyId === passport.downloadId} onClick={() => void exportPassport(passport)}>Export passport</button>
              <button type="button" className="secondary" disabled={busyId === passport.downloadId} onClick={() => void runFor(passport.downloadId, "import_os_provenance")}>Import OS provenance</button>
            </div>

            {passport.localState === "LOCAL_MISSING" ? (
              <div className="relink-row">
                <input value={relink[passport.downloadId] || ""} onChange={(event) => setRelink((values) => ({ ...values, [passport.downloadId]: event.target.value }))} placeholder="New full path for this file" />
                <button type="button" className="secondary" disabled={busyId === passport.downloadId} onClick={() => void runFor(passport.downloadId, "relink_file", { newPath: relink[passport.downloadId] || "" })}>Relink by SHA-256</button>
              </div>
            ) : null}

            <details>
              <summary>Location history ({passport.locations.length})</summary>
              <ul className="passport-locations">
                {passport.locations.map((location) => <li key={location.path}>{location.isCurrent ? "Current" : "Previous"}: {location.path}</li>)}
              </ul>
            </details>
          </article>
        ))}
      </div>

      {trust ? (
        <section className="trust-lens" aria-live="polite">
          <div className="section-heading"><div><p className="eyebrow">Evidence, not a safety score</p><h2>Trust Lens · {trust.fileName}</h2></div><button className="secondary" type="button" onClick={() => setTrust(null)}>Close</button></div>
          <div className="trust-grid">
            {trust.evidence.map((item) => (
              <article key={item.kind}>
                <div className="badge-row"><strong>{readable(item.kind)}</strong><span className="status">{readable(item.state)}</span></div>
                <p>{item.summary}</p>
                {item.details.length ? <ul>{item.details.map((detail) => <li key={detail}>{detail}</li>)}</ul> : null}
              </article>
            ))}
          </div>
        </section>
      ) : null}

      <details className="origin-graph">
        <summary>Origin Graph · {graph.nodes.length} nodes / {graph.edges.length} evidence links</summary>
        <div className="graph-columns">
          <div><h3>Origins and files</h3>{graph.nodes.map((node) => <p key={node.id}><strong>{node.kind}</strong> · {node.label}{node.state ? ` · ${node.state}` : ""}</p>)}</div>
          <div><h3>Relationships</h3>{graph.edges.map((edge, index) => <p key={`${edge.from}-${edge.to}-${index}`}>{edge.from} → <strong>{readable(edge.relation)}</strong> → {edge.to}</p>)}</div>
        </div>
      </details>
    </section>
  );
}
