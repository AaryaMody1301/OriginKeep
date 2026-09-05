import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./passport.css";

type FileLocation = {
  path: string;
  isCurrent: boolean;
  firstSeen: string;
  lastSeen: string;
};

type PassportRecord = {
  downloadId: number;
  fileName: string;
  localPath: string;
  mimeType: string | null;
  bytes: number | null;
  sha256: string | null;
  status: string;
  sourceIdentity: string | null;
  versionNumber: number | null;
  duplicateOfId: number | null;
  localState: string;
  originalUrl: string;
  finalUrl: string | null;
  referrer: string | null;
  pageUrl: string | null;
  pageTitle: string | null;
  linkText: string | null;
  contextText: string | null;
  browserName: string | null;
  completedAt: string | null;
  purpose: string | null;
  note: string | null;
  expiresAt: string | null;
  retentionPolicy: string;
  latestRemoteState: string | null;
  latestRemoteCheckedAt: string | null;
  lifecycleState: string;
  locations: FileLocation[];
  portablePassportPath: string | null;
};

type OriginNode = { id: string; kind: string; label: string; detail: string | null };
type OriginEdge = { from: string; to: string; kind: string };
type OriginGraph = { nodes: OriginNode[]; edges: OriginEdge[] };

type TrustEvidence = { state: string; summary: string; detail: string | null };
type TrustLens = {
  downloadId: number;
  fileName: string;
  integrity: TrustEvidence;
  origin: TrustEvidence;
  platformOrigin: TrustEvidence;
  publisherSignature: TrustEvidence;
  c2pa: TrustEvidence;
  sigstore: TrustEvidence;
};

type BridgeStatus = {
  platform: string;
  nativeHostPath: string | null;
  targets: Array<{ browser: string; manifestPath: string | null; state: string; detail: string }>;
  safariNote: string;
};

type RelinkResult = {
  downloadId: number;
  found: boolean;
  scannedEntries: number;
  path: string | null;
  message: string;
};

type SigstoreVerification = {
  downloadId: number;
  bundlePath: string;
  identity: string;
  issuer: string;
  state: string;
  evidence: string;
};

const PURPOSES = ["Reference", "Read later", "Temporary", "Work", "Receipt", "Installer", "Dataset", "Other"];
const RETENTION = [
  ["MANUAL", "Manual"],
  ["REVIEW_WHEN_NEWER", "Review when source changes"],
  ["ARCHIVE_WHEN_SUPERSEDED", "Archive candidate when superseded"],
  ["ARCHIVE_WHEN_EXPIRED", "Archive candidate after expiry"],
  ["NEVER_ARCHIVE", "Never archive"],
];

function readable(value: string | null | undefined) {
  return value ? value.replaceAll("_", " ") : "Unknown";
}

function shortHash(value: string | null) {
  return value ? `${value.slice(0, 16)}…` : "No fingerprint";
}

function evidenceClass(state: string) {
  const positive = ["MATCH", "RECORDED", "VALID", "VERIFIED", "CRYPTOGRAPHIC_VALIDATION_PASSED", "WINDOWS_MOTW_PRESENT", "MACOS_WHEREFROMS_PRESENT"];
  const negative = ["INVALID", "LOCAL_MODIFIED", "VERIFICATION_FAILED", "CHECK_FAILED"];
  if (positive.includes(state)) return "trust-good";
  if (negative.includes(state)) return "trust-bad";
  return "trust-neutral";
}

function TrustRow({ label, value }: { label: string; value: TrustEvidence }) {
  return (
    <div className={`trust-row ${evidenceClass(value.state)}`}>
      <div><strong>{label}</strong><span>{readable(value.state)}</span></div>
      <p>{value.summary}</p>
      {value.detail ? <small>{value.detail}</small> : null}
    </div>
  );
}

function PassportCard({ record, onChanged }: { record: PassportRecord; onChanged: () => Promise<void> }) {
  const [purpose, setPurpose] = useState(record.purpose ?? "");
  const [note, setNote] = useState(record.note ?? "");
  const [expiresAt, setExpiresAt] = useState(record.expiresAt ?? "");
  const [retentionPolicy, setRetentionPolicy] = useState(record.retentionPolicy);
  const [trust, setTrust] = useState<TrustLens | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  async function run(label: string, action: () => Promise<string>, refresh = false) {
    setBusy(label);
    setMessage(null);
    try {
      setMessage(await action());
      if (refresh) await onChanged();
    } catch (cause) {
      setMessage(`Error: ${cause instanceof Error ? cause.message : String(cause)}`);
    } finally {
      setBusy(null);
    }
  }

  async function saveMetadata(event: FormEvent) {
    event.preventDefault();
    await run(
      "save",
      async () => {
        await invoke<PassportRecord>("update_passport_metadata", {
          downloadId: record.downloadId,
          purpose: purpose.trim() || null,
          note: note.trim() || null,
          expiresAt: expiresAt.trim() || null,
          retentionPolicy,
        });
        return "Passport intent saved. The safe lifecycle review now respects this policy.";
      },
      true,
    );
  }

  async function inspectTrust() {
    await run("trust", async () => {
      const result = await invoke<TrustLens>("inspect_trust", { downloadId: record.downloadId });
      setTrust(result);
      return "Trust evidence refreshed from the current local file and available platform/verifier evidence.";
    });
  }

  async function exportPortable() {
    await run(
      "export",
      async () => {
        const result = await invoke<{ path: string; sha256: string }>("export_passport", { downloadId: record.downloadId });
        return `Portable passport written beside the verified file: ${result.path}`;
      },
      true,
    );
  }

  async function relink() {
    const candidatePath = window.prompt("Exact path to the moved/renamed file. OriginKeep will accept it only if SHA-256 matches.");
    if (!candidatePath) return;
    await run(
      "relink",
      async () => {
        const result = await invoke<PassportRecord>("relink_download", { downloadId: record.downloadId, candidatePath });
        return `Relinked by exact content identity: ${result.localPath}`;
      },
      true,
    );
  }

  async function findMoved() {
    const searchRoot = window.prompt("Directory to scan for the same exact file. The scan is bounded to 20,000 entries and depth 8.");
    if (!searchRoot) return;
    await run(
      "find",
      async () => {
        const result = await invoke<RelinkResult>("find_moved_file", { downloadId: record.downloadId, searchRoot });
        return `${result.message} Scanned ${result.scannedEntries} entries.`;
      },
      true,
    );
  }

  async function verifySigstore() {
    const identity = window.prompt("Expected Sigstore certificate identity (exact signer identity or workflow identity):");
    if (!identity) return;
    const issuer = window.prompt("Expected Sigstore OIDC issuer, e.g. https://token.actions.githubusercontent.com:");
    if (!issuer) return;
    await run("sigstore", async () => {
      const result = await invoke<SigstoreVerification>("verify_sigstore", {
        downloadId: record.downloadId,
        identity,
        issuer,
      });
      return `${readable(result.state)} — ${result.evidence || result.bundlePath}`;
    });
  }

  return (
    <article className="passport-card">
      <div className="passport-title-row">
        <div>
          <p className="passport-kicker">File Passport #{record.downloadId}</p>
          <h3>{record.fileName}</h3>
          <p className="passport-path">{record.localPath}</p>
        </div>
        <div className="passport-badges">
          {record.versionNumber !== null ? <span>v{record.versionNumber}</span> : null}
          <span>{readable(record.status)}</span>
          <span>{readable(record.localState)}</span>
          <span>{readable(record.lifecycleState)}</span>
        </div>
      </div>

      <div className="passport-grid">
        <section>
          <h4>Origin & context</h4>
          <dl>
            <div><dt>Source</dt><dd>{record.sourceIdentity || record.originalUrl}</dd></div>
            <div><dt>Source page</dt><dd>{record.pageTitle || record.pageUrl || record.referrer || "Not captured"}</dd></div>
            <div><dt>Link</dt><dd>{record.linkText || "Not captured"}</dd></div>
            <div><dt>Nearby context</dt><dd>{record.contextText || "Rich context was not enabled/captured for this download."}</dd></div>
            <div><dt>Browser</dt><dd>{record.browserName || "Unknown / imported passport"}</dd></div>
            <div><dt>Fingerprint</dt><dd>{shortHash(record.sha256)}</dd></div>
            <div><dt>Remote</dt><dd>{readable(record.latestRemoteState)}{record.latestRemoteCheckedAt ? ` · ${record.latestRemoteCheckedAt}` : ""}</dd></div>
          </dl>
        </section>

        <section>
          <h4>Identity & locations</h4>
          <p>{record.locations.length} known path{record.locations.length === 1 ? "" : "s"}. Identity follows SHA-256, not the filename.</p>
          <ul className="location-list">
            {record.locations.map((location) => (
              <li key={location.path}>
                <strong>{location.isCurrent ? "Current" : "Previous"}</strong> {location.path}
              </li>
            ))}
          </ul>
          {record.portablePassportPath ? <p className="portable-note">Portable passport: {record.portablePassportPath}</p> : null}
          <div className="passport-actions">
            <button type="button" onClick={() => void exportPortable()} disabled={busy !== null}>{busy === "export" ? "Exporting…" : "Export passport"}</button>
            <button type="button" className="secondary" onClick={() => void relink()} disabled={busy !== null}>Relink exact file</button>
            <button type="button" className="secondary" onClick={() => void findMoved()} disabled={busy !== null}>{busy === "find" ? "Scanning…" : "Find moved file"}</button>
          </div>
        </section>
      </div>

      <form className="passport-intent" onSubmit={saveMetadata}>
        <label>
          Why did I save this?
          <select value={purpose} onChange={(event) => setPurpose(event.target.value)}>
            <option value="">Unspecified</option>
            {PURPOSES.map((value) => <option key={value} value={value}>{value}</option>)}
          </select>
        </label>
        <label>
          Review/expiry time
          <input value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} placeholder="2026-12-31T18:00:00Z" />
        </label>
        <label>
          Lifecycle intent
          <select value={retentionPolicy} onChange={(event) => setRetentionPolicy(event.target.value)}>
            {RETENTION.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select>
        </label>
        <label className="passport-note-field">
          Note
          <textarea value={note} onChange={(event) => setNote(event.target.value)} placeholder="Why this file matters, what it is for, or what to remember later." />
        </label>
        <button type="submit" disabled={busy !== null}>{busy === "save" ? "Saving…" : "Save passport intent"}</button>
      </form>

      <div className="trust-actions">
        <button type="button" className="secondary" onClick={() => void inspectTrust()} disabled={busy !== null}>{busy === "trust" ? "Inspecting…" : "Inspect Trust Lens"}</button>
        <button type="button" className="secondary" onClick={() => void verifySigstore()} disabled={busy !== null}>Verify Sigstore bundle</button>
      </div>
      {trust ? (
        <section className="trust-lens">
          <h4>Trust Lens — evidence, not a score</h4>
          <TrustRow label="Local integrity" value={trust.integrity} />
          <TrustRow label="Recorded origin" value={trust.origin} />
          <TrustRow label="Platform origin" value={trust.platformOrigin} />
          <TrustRow label="Publisher signature" value={trust.publisherSignature} />
          <TrustRow label="C2PA" value={trust.c2pa} />
          <TrustRow label="Sigstore" value={trust.sigstore} />
        </section>
      ) : null}
      {message ? <p className={`passport-message ${message.startsWith("Error:") ? "error" : ""}`}>{message}</p> : null}
    </article>
  );
}

export default function PassportDashboard() {
  const [passports, setPassports] = useState<PassportRecord[]>([]);
  const [graph, setGraph] = useState<OriginGraph>({ nodes: [], edges: [] });
  const [bridge, setBridge] = useState<BridgeStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [importPassportPath, setImportPassportPath] = useState("");
  const [importFilePath, setImportFilePath] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [rows, originGraph, bridgeStatus] = await Promise.all([
        invoke<PassportRecord[]>("list_passports"),
        invoke<OriginGraph>("origin_graph"),
        invoke<BridgeStatus>("browser_bridge_status"),
      ]);
      setPassports(rows);
      setGraph(originGraph);
      setBridge(bridgeStatus);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function importPortable(event: FormEvent) {
    event.preventDefault();
    setError(null);
    try {
      await invoke<PassportRecord>("import_passport", {
        passportPath: importPassportPath.trim(),
        filePath: importFilePath.trim(),
      });
      setImportPassportPath("");
      setImportFilePath("");
      await load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  const sites = useMemo(() => graph.nodes.filter((node) => node.kind === "SITE"), [graph]);
  const sources = useMemo(() => graph.nodes.filter((node) => node.kind === "SOURCE"), [graph]);
  const files = useMemo(() => graph.nodes.filter((node) => node.kind === "FILE"), [graph]);
  const contextRich = passports.filter((item) => item.pageTitle || item.linkText || item.contextText).length;
  const moved = passports.filter((item) => item.locations.length > 1).length;
  const portable = passports.filter((item) => item.portablePassportPath).length;

  return (
    <section className="passport-shell" aria-labelledby="passport-workspace-title">
      <header className="passport-hero">
        <div>
          <p className="passport-kicker">OriginKeep 2.0 · Universal File Passport</p>
          <h2 id="passport-workspace-title">Every file remembers its story.</h2>
          <p>Origin, context, content identity, integrity, authenticity evidence, freshness, lineage, intent and recovery — kept locally around the file you actually own.</p>
        </div>
        <button type="button" className="secondary" onClick={() => void load()}>Refresh passports</button>
      </header>

      <div className="passport-metrics">
        <div><strong>{passports.length}</strong><span>file passports</span></div>
        <div><strong>{contextRich}</strong><span>with save context</span></div>
        <div><strong>{moved}</strong><span>with path history</span></div>
        <div><strong>{portable}</strong><span>portable exports</span></div>
        <div><strong>{sites.length}</strong><span>origin sites</span></div>
      </div>

      {error ? <p className="passport-message error">OriginKeep Passport: {error}</p> : null}

      <div className="passport-top-grid">
        <form className="passport-import" onSubmit={importPortable}>
          <h3>Import a portable passport</h3>
          <p>Choose a `.originkeep.json` passport and the file it describes. Import succeeds only when SHA-256 matches.</p>
          <label>Passport JSON path<input value={importPassportPath} onChange={(event) => setImportPassportPath(event.target.value)} required /></label>
          <label>File path<input value={importFilePath} onChange={(event) => setImportFilePath(event.target.value)} required /></label>
          <button type="submit">Verify & import</button>
        </form>

        <section className="bridge-panel">
          <h3>Browser bridge</h3>
          <p>{bridge ? `${bridge.platform} · ${bridge.nativeHostPath || "native host not located"}` : "Checking local browser bridge…"}</p>
          <ul>
            {bridge?.targets.map((target) => <li key={target.browser}><strong>{target.browser}</strong> · {readable(target.state)}<small>{target.detail}</small></li>)}
          </ul>
          {bridge ? <p className="safari-note">{bridge.safariNote}</p> : null}
        </section>
      </div>

      <section className="origin-graph-panel">
        <div>
          <p className="passport-kicker">Origin Graph</p>
          <h3>{sites.length} sites → {sources.length} sources → {files.length} tracked files</h3>
          <p>{graph.edges.filter((edge) => edge.kind === "NEXT_VERSION").length} version transitions · {graph.edges.filter((edge) => edge.kind === "EXACT_DUPLICATE_OF").length} exact-duplicate relationships</p>
        </div>
        <div className="origin-columns">
          <div><strong>Sites</strong>{sites.slice(0, 8).map((node) => <span key={node.id}>{node.label}</span>)}</div>
          <div><strong>Sources</strong>{sources.slice(0, 8).map((node) => <span key={node.id}>{node.label}</span>)}</div>
          <div><strong>Files</strong>{files.slice(0, 8).map((node) => <span key={node.id}>{node.label}{node.detail ? ` · ${node.detail}` : ""}</span>)}</div>
        </div>
      </section>

      <div className="passport-list">
        {loading ? <p>Loading File Passports…</p> : null}
        {!loading && passports.length === 0 ? <p>No passports yet. Existing and future tracked downloads will appear here automatically.</p> : null}
        {passports.map((record) => <PassportCard key={record.downloadId} record={record} onChanged={load} />)}
      </div>
    </section>
  );
}
