import { FormEvent, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function AdoptExisting() {
  const [filePath, setFilePath] = useState("");
  const [sourceUrl, setSourceUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!filePath.trim()) return;
    setBusy(true);
    setMessage(null);
    try {
      const result = await invoke<{ fileName: string; sha256: string | null }>(
        "adopt_existing_file",
        {
          filePath: filePath.trim(),
          sourceUrl: sourceUrl.trim() || null,
        },
      );
      setMessage(`Adopted ${result.fileName} with ${result.sha256 ? "verified SHA-256 identity" : "no fingerprint"}. Refreshing…`);
      window.setTimeout(() => window.location.reload(), 700);
    } catch (cause) {
      setMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <aside className="v2-adopt" aria-label="Adopt an existing file">
      <div>
        <p className="v2-eyebrow">Files from before OriginKeep</p>
        <strong>Adopt existing file</strong>
        <small>Windows/macOS provenance is imported when the OS still has it. Otherwise the source stays unknown unless you provide one.</small>
      </div>
      <form onSubmit={submit}>
        <input value={filePath} onChange={(event) => setFilePath(event.target.value)} placeholder="Absolute file path" />
        <input value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} placeholder="Optional known source URL" />
        <button type="submit" disabled={busy}>{busy ? "Hashing…" : "Adopt file"}</button>
      </form>
      {message ? <small className="v2-adopt-message">{message}</small> : null}
    </aside>
  );
}
