import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type BrowserSetupResult = {
  platform: string;
  nativeHostPath: string;
  manifests: string[];
  note: string;
};

export default function BrowserSetupBar() {
  const [result, setResult] = useState<BrowserSetupResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function register() {
    setBusy(true);
    setError(null);
    try {
      const next = await invoke<BrowserSetupResult>("register_browser_integrations");
      setResult(next);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="browser-setup-bar">
      <div>
        <p className="eyebrow">Browser reach</p>
        <strong>Chrome · Edge · Firefox integrations</strong>
        <p>Windows registration is installed by NSIS. macOS and Linux can register the bundled native host here for the current user.</p>
      </div>
      <button type="button" className="secondary" disabled={busy} onClick={() => void register()}>
        {busy ? "Registering…" : "Register browser integrations"}
      </button>
      {result ? (
        <div className="browser-setup-result">
          <p><strong>{result.platform}</strong> · {result.note}</p>
          <p className="path">Native host: {result.nativeHostPath}</p>
          {result.manifests.map((manifest) => <p className="path" key={manifest}>Manifest: {manifest}</p>)}
        </div>
      ) : null}
      {error ? <p className="error">Browser setup failed: {error}</p> : null}
    </section>
  );
}
