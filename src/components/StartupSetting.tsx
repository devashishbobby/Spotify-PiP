import { useEffect, useState } from "react";
import { command, desktop, subscribe } from "../lib/desktop";

export function StartupSetting() {
  const [enabled, setEnabled] = useState(true);
  const [loading, setLoading] = useState(desktop);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    if (desktop) {
      void command<boolean>("get_startup_enabled")
        .then((value) => { if (!disposed) setEnabled(value); })
        .catch((reason: unknown) => { if (!disposed) setError(String(reason)); })
        .finally(() => { if (!disposed) setLoading(false); });
    }
    const unsubscribe = subscribe<boolean>("desktop://startup", setEnabled);
    return () => { disposed = true; unsubscribe(); };
  }, []);

  async function change(next: boolean) {
    setPending(true);
    setError(null);
    try {
      setEnabled(await command<boolean>("set_startup_enabled", { enabled: next }));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <div className="setting-row">
        <label htmlFor="windows-startup" title="Launch in the tray when you sign in to Windows.">
          Start when Windows boots up
        </label>
        <input
          id="windows-startup"
          type="checkbox"
          role="switch"
          checked={enabled}
          disabled={!desktop || loading || pending}
          aria-busy={loading || pending}
          aria-describedby={error ? "startup-error" : undefined}
          onChange={(event) => void change(event.target.checked)}
        />
      </div>
      {error && <p id="startup-error" className="shortcut-error" role="alert">{error}</p>}
    </>
  );
}
