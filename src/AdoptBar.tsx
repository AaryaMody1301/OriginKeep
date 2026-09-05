import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function AdoptBar() {
  const [localPath, setLocalPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function adopt() {
    if (!localPath.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("adopt_file", { localPath: localPath.trim() });
      window.location.reload();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setBusy(false);
    }
  }

  return (
    <section className="adopt-bar">
      <div>
        <p className="eyebrow">Chrome · Edge · Firefox · Safari/macOS fallback · existing files</p>
        <strong>Adopt any local file into OriginKeep</strong>
        <p>OriginKeep fingerprints the bytes first and imports available OS provenance. This is also the Safari/macOS bridge because Safari does not expose the WebExtensions downloads API.</p>
      </div>
      <div className="adopt-controls">
        <input value={localPath} onChange={(event) => setLocalPath(event.target.value)} placeholder="Full path to an existing file" />
        <button type="button" disabled={busy} onClick={() => void adopt()}>{busy ? "Adopting…" : "Create passport"}</button>
      </div>
      {error ? <p className="error">Could not adopt file: {error}</p> : null}
    </section>
  );
}
