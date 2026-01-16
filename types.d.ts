type ClientEvent = import("./src/ui/types").ClientEvent;
type ServerEvent = import("./src/ui/types").ServerEvent;

interface Window {
    __TAURI__?: {
        core?: {
            invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
        event?: {
            listen: (
                event: string,
                handler: (event: { payload: unknown }) => void
            ) => Promise<() => void>;
        };
    };
}
