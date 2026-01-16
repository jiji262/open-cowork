import { useCallback, useEffect, useRef, useState } from "react";
import type { ServerEvent, ClientEvent } from "../types";

export function useIPC(onEvent: (event: ServerEvent) => void) {
  const unsubscribeRef = useRef<(() => void) | null>(null);
  const tauri = typeof window !== "undefined" ? window.__TAURI__ : undefined;
  const [connected, setConnected] = useState(Boolean(tauri?.event?.listen));

  useEffect(() => {
    let active = true;

    const tauriEvent = tauri?.event;
    if (tauriEvent?.listen) {
      setConnected(true);
      (async () => {
        const unlisten = await tauriEvent.listen("server-event", (event) => {
          if (!active) return;
          onEvent(event.payload as ServerEvent);
        });
        if (!active) {
          unlisten();
          return;
        }
        unsubscribeRef.current = unlisten;
      })();
    }

    return () => {
      active = false;
      setConnected(false);
      if (unsubscribeRef.current) {
        unsubscribeRef.current();
        unsubscribeRef.current = null;
      }
    };
  }, [onEvent, tauri]);

  const sendEvent = useCallback((event: ClientEvent) => {
    if (tauri?.core?.invoke) {
      void tauri.core.invoke("client_event", { event });
    }
  }, [tauri]);

  return { connected, sendEvent };
}
