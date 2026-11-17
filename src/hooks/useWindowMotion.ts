import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { command, desktop, type DesktopStatus } from "../lib/desktop";

interface WindowTransition {
  revision: number;
  visible: boolean;
  animate: boolean;
  wasVisible: boolean;
}
function validTransition(value: unknown): value is WindowTransition {
  if (!value || typeof value !== "object") return false;
  const state = value as Partial<WindowTransition>;
  return Number.isSafeInteger(state.revision) && state.revision! >= 0 &&
    typeof state.visible === "boolean" && typeof state.animate === "boolean" &&
    typeof state.wasVisible === "boolean";
}

// Only the panel's transient presentation changes; native position and controls
// remain untouched. Native revision guards and a 280ms timeout own reliability.
export function useWindowMotion() {
  const panel = useRef<HTMLElement>(null);
  useEffect(() => {
    const element = panel.current;
    if (!desktop || !element) return;
    let disposed = false;
    let latest: WindowTransition | undefined;
    let animation: Animation | undefined;
    const stops: (() => void)[] = [];
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const original = {
      opacity: element.style.opacity,
      transform: element.style.transform,
      origin: element.style.transformOrigin,
    };
    element.style.transformOrigin = "50% 100%";
    const hidden = "translateY(4px) scale(0.985)";

    function acknowledge(revision: number) {
      void command("window_transition_complete", { revision }).catch(() => {
        // Native fallback owns completion if the bridge disappears during reload.
      });
    }
    function settle(state: WindowTransition, acknowledgeCompletion: boolean) {
      animation?.cancel();
      animation = undefined;
      element!.style.opacity = state.visible ? "1" : "0";
      element!.style.transform = state.visible ? "none" : hidden;
      element!.dataset.windowMotion = state.visible ? "shown" : "hidden";
      if (acknowledgeCompletion) acknowledge(state.revision);
    }
    function apply(value: unknown) {
      if (disposed || !validTransition(value)) return;
      if (latest && value.revision < latest.revision) return;
      latest = value;
      element!.dataset.windowRevision = String(value.revision);
      if (!value.animate || reduced.matches) {
        settle(value, value.animate);
        return;
      }
      // Capture the currently rendered frame before cancelling: reversals continue
      // from that frame rather than snapping back to a fixed keyframe.
      const current = getComputedStyle(element!);
      const from = {
        opacity: value.visible && !value.wasVisible ? "0" : current.opacity,
        transform: value.visible && !value.wasVisible ? hidden : current.transform,
      };
      animation?.cancel();
      element!.style.opacity = from.opacity;
      element!.style.transform = from.transform;
      element!.dataset.windowMotion = value.visible ? "showing" : "hiding";
      const next = element!.animate([
        from,
        { opacity: value.visible ? "1" : "0", transform: value.visible ? "none" : hidden },
      ], {
        duration: value.visible ? 200 : 140,
        easing: "cubic-bezier(0.2, 0.7, 0.2, 1)",
        fill: "forwards",
      });
      animation = next;
      void next.finished.then(() => {
        if (!disposed && animation === next && latest?.revision === value.revision) {
          settle(value, true);
        }
      }).catch(() => { /* A reversal or unmount deliberately cancels this animation. */ });
    }
    async function ready() {
      const state = await command<unknown>("window_motion_ready", { reducedMotion: reduced.matches });
      apply(state);
    }
    const preferenceChanged = () => {
      if (reduced.matches && latest) settle(latest, latest.animate);
      void ready().catch(() => { /* A native timeout remains available. */ });
    };
    reduced.addEventListener("change", preferenceChanged);
    void (async () => {
      // Listen before synchronizing so an in-flight hotkey cannot be lost.
      const transitionStop = await listen<WindowTransition>("desktop://transition", ({ payload }) => apply(payload));
      if (disposed) { transitionStop(); return; }
      stops.push(transitionStop);
      const statusStop = await listen<DesktopStatus>("desktop://status", ({ payload }) => {
        if (!disposed && !payload.visible && latest && !latest.visible) settle(latest, false);
      });
      if (disposed) { statusStop(); return; }
      stops.push(statusStop);
      await ready();
    })().catch(() => { /* A missing WebView must not prevent native show/hide. */ });
    return () => {
      disposed = true;
      reduced.removeEventListener("change", preferenceChanged);
      stops.forEach((stop) => stop());
      animation?.cancel();
      element.style.opacity = original.opacity;
      element.style.transform = original.transform;
      element.style.transformOrigin = original.origin;
      delete element.dataset.windowMotion;
      delete element.dataset.windowRevision;
    };
  }, []);
  return panel;
}
