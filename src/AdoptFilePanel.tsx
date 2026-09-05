import { FormEvent, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function AdoptFilePanel() {
  const [filePath, setFilePath] = useState("");
  const [sourceUrl, setSourceUrl] = useState("");
  const [purpose, setPurpose] = useState("");
  const [note, setNote] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setMessage(null);
    try {
      await invoke("adopt_local_file", {
        filePath: filePath.trim(),
        sourceUrl: sourceUrl.trim() || null,
        purpose: purpose.trim() || null,
        note: note.trim() || null,
      });
      setMessage("File adopted by content fingerprint. Reloading its new Passport…");
      window.location.reload();
    } catch (cause) {
      setMessage(`Error: ${cause instanceof Error ? cause.message : String(cause)}`);
      setBusy(false);
    }
  }

  return (
    <section className="passport-shell adopt-shell" aria-labelledby="adopt-title">
      <form className="passport-import adopt-panel" onSubmit={submit}>
        <div>
          <p className="passport-kicker">Universal adoption</p>
          <h3 id="adopt-title">Give any existing local file a Passport</h3>
          <p>OriginKeep fingerprints the existing bytes first. A source URL is optional; when omitted, origin remains explicitly local/unknown rather than being invented.</p>
        </div>
        <label>Local file path<input value={filePath} onChange={(event) => setFilePath(event.target.value)} required /></label>
        <label>Known source URL (optional)<input value={sourceUrl} onChange={(event) => setSourceUrl(event.target.value)} placeholder="https://…" /></label>
        <label>Purpose (optional)<input value={purpose} onChange={(event) => setPurpose(event.target.value)} placeholder="Reference, Work, Receipt, Dataset…" /></label>
        <label>Note (optional)<input value={note} onChange={(event) => setNote(event.target.value)} /></label>
        <button type="submit" disabled={busy}>{busy ? "Adopting…" : "Create File Passport"}</button>
        {message ? <p className={`passport-message ${message.startsWith("Error:") ? "error" : ""}`}>{message}</p> : null}
      </form>
    </section>
  );
}
