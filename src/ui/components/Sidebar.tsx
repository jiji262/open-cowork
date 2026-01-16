import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import * as Dialog from "@radix-ui/react-dialog";
import { useAppStore } from "../store/useAppStore";
import { ProviderSettings } from "./ProviderSettings";

interface SidebarProps {
  connected: boolean;
  onNewSession: () => void;
  onDeleteSession: (sessionId: string) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
}

export function Sidebar({
  onNewSession,
  onDeleteSession,
  connected,
  collapsed,
  onToggleCollapse
}: SidebarProps) {
  const sessions = useAppStore((state) => state.sessions);
  const activeSessionId = useAppStore((state) => state.activeSessionId);
  const setActiveSessionId = useAppStore((state) => state.setActiveSessionId);
  const activeProvider = useAppStore((state) => state.activeProvider);
  const setActiveProvider = useAppStore((state) => state.setActiveProvider);
  const providerConfigs = useAppStore((state) => state.providerConfigs);
  const setProviderConfig = useAppStore((state) => state.setProviderConfig);
  const permissionMode = useAppStore((state) => state.permissionMode);
  const setPermissionMode = useAppStore((state) => state.setPermissionMode);
  const [resumeSessionId, setResumeSessionId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const closeTimerRef = useRef<number | null>(null);

  const formatCwd = (cwd?: string) => {
    if (!cwd) return "Working dir unavailable";
    const parts = cwd.split(/[\\/]+/).filter(Boolean);
    const tail = parts.slice(-2).join("/");
    return `/${tail || cwd}`;
  };

  const sessionList = useMemo(() => {
    const list = Object.values(sessions);
    list.sort((a, b) => (b.updatedAt ?? 0) - (a.updatedAt ?? 0));
    return list;
  }, [sessions]);

  const resetResumeSession = useCallback((nextId: string | null) => {
    setResumeSessionId(nextId);
    setCopied(false);
    if (closeTimerRef.current) {
      window.clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  }, []);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current) {
        window.clearTimeout(closeTimerRef.current);
        closeTimerRef.current = null;
      }
    };
  }, []);

  const handleCopyCommand = async () => {
    if (!resumeSessionId) return;
    const command = `claude --resume ${resumeSessionId}`;
    try {
      await navigator.clipboard.writeText(command);
    } catch {
      return;
    }
    setCopied(true);
    if (closeTimerRef.current) {
      window.clearTimeout(closeTimerRef.current);
    }
    closeTimerRef.current = window.setTimeout(() => {
      resetResumeSession(null);
    }, 3000);
  };

  const getStatusColor = (status: string) => {
    if (status === "running") return "bg-info";
    if (status === "completed") return "bg-success";
    if (status === "error") return "bg-error";
    return "bg-ink-400";
  };

  return (
    <aside
      className={`relative z-20 flex h-full shrink-0 flex-col gap-4 border-r border-ink-900/5 bg-surface-cream pb-4 pt-12 transition-[width] duration-200 ${collapsed ? "px-2" : "px-4"}`}
      style={{ width: "var(--sidebar-width)" } as React.CSSProperties}
    >
      <div
        className="absolute top-0 left-0 right-0 h-12"
        style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
      />
      <div className={`flex items-center ${collapsed ? "justify-center" : "justify-between"} gap-2`}>
        <div className={`flex items-center gap-2 ${collapsed ? "justify-center" : ""}`}>
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-accent/10 text-sm font-semibold text-accent">
            oc
          </div>
          {!collapsed && <span className="text-sm font-semibold text-ink-800">open-cowork</span>}
        </div>
        <div className="flex items-center gap-2">
          <Dialog.Root open={settingsOpen} onOpenChange={setSettingsOpen}>
            <Dialog.Trigger asChild>
              <button
                type="button"
                className="flex h-8 w-8 items-center justify-center rounded-full border border-ink-900/10 bg-surface text-ink-600 hover:bg-surface-tertiary hover:text-ink-800 transition-colors"
                aria-label="Open settings"
              >
                <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8">
                  <path d="M12 3v2m0 14v2m9-9h-2M5 12H3m15.36 6.36-1.41-1.41M7.05 7.05 5.64 5.64m12.72 0-1.41 1.41M7.05 16.95l-1.41 1.41" />
                  <circle cx="12" cy="12" r="3.5" />
                </svg>
              </button>
            </Dialog.Trigger>
            <Dialog.Portal>
              <Dialog.Overlay className="fixed inset-0 z-50 bg-ink-900/40 backdrop-blur-sm" />
              <Dialog.Content className="fixed left-1/2 top-1/2 z-[60] w-full max-w-lg -translate-x-1/2 -translate-y-1/2 rounded-2xl bg-white p-6 shadow-xl">
                <div className="flex items-start justify-between gap-4">
                  <Dialog.Title className="text-lg font-semibold text-ink-800">Settings</Dialog.Title>
                  <Dialog.Close asChild>
                    <button className="rounded-full p-1 text-ink-500 hover:bg-ink-900/10" aria-label="Close dialog">
                      <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M6 6l12 12M18 6l-12 12" />
                      </svg>
                    </button>
                  </Dialog.Close>
                </div>
                <div className="mt-4">
                  <ProviderSettings
                    value={activeProvider}
                    onChange={setActiveProvider}
                    config={providerConfigs[activeProvider]}
                    onConfigChange={(config) => setProviderConfig(activeProvider, config)}
                    permissionMode={permissionMode}
                    onPermissionModeChange={setPermissionMode}
                  />
                </div>
              </Dialog.Content>
            </Dialog.Portal>
          </Dialog.Root>
          <button
            className="rounded-full border border-ink-900/10 bg-surface p-1.5 text-ink-600 hover:bg-surface-tertiary transition-colors"
            onClick={onToggleCollapse}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            <svg viewBox="0 0 24 24" className={`h-4 w-4 transition-transform ${collapsed ? "rotate-180" : ""}`} fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M15 6l-6 6 6 6" />
            </svg>
          </button>
        </div>
      </div>
      <button
        className={`w-full rounded-xl border border-ink-900/10 bg-surface py-2.5 text-sm font-medium text-ink-700 hover:bg-surface-tertiary hover:border-ink-900/20 transition-colors ${collapsed ? "px-2" : "px-4"}`}
        onClick={onNewSession}
        aria-label="New Task"
      >
        {collapsed ? "+" : "+ New Task"}
      </button>
      <div className="flex flex-1 flex-col gap-2 overflow-y-auto">
        {sessionList.length === 0 && (
          <div className={`rounded-xl border border-ink-900/5 bg-surface px-4 py-5 text-center text-xs text-muted ${collapsed ? "px-2" : ""}`}>
            {collapsed ? "No sessions" : "No sessions yet. Start by sending a prompt."}
          </div>
        )}
        {sessionList.map((session) => (
          <div
            key={session.id}
            className={`cursor-pointer rounded-xl border text-left transition ${activeSessionId === session.id ? "border-accent/30 bg-accent-subtle" : "border-ink-900/5 bg-surface hover:bg-surface-tertiary"} ${collapsed ? "px-2 py-2" : "px-2 py-3"}`}
            onClick={() => setActiveSessionId(session.id)}
            onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setActiveSessionId(session.id); } }}
            role="button"
            tabIndex={0}
            title={session.title}
          >
            {collapsed ? (
              <div className="flex items-center justify-center">
                <div className="relative flex h-9 w-9 items-center justify-center rounded-xl bg-surface-tertiary text-xs font-semibold text-ink-700">
                  {(session.title || "•").trim().slice(0, 1).toUpperCase()}
                  <span className={`absolute -bottom-1 -right-1 h-2.5 w-2.5 rounded-full border-2 border-surface ${getStatusColor(session.status)}`} />
                </div>
              </div>
            ) : (
              <div className="flex items-center justify-between gap-2">
                <div className="flex flex-col min-w-0 flex-1 overflow-hidden">
                  <div className={`text-[12px] font-medium ${session.status === "running" ? "text-info" : session.status === "completed" ? "text-success" : session.status === "error" ? "text-error" : "text-ink-800"}`}>
                    {session.title}
                  </div>
                  <div className="flex items-center justify-between mt-0.5 text-xs text-muted">
                    <span className="truncate">{formatCwd(session.cwd)}</span>
                  </div>
                </div>
                <DropdownMenu.Root>
                  <DropdownMenu.Trigger asChild>
                    <button className="flex-shrink-0 rounded-full p-1.5 text-ink-500 hover:bg-ink-900/10" aria-label="Open session menu" onClick={(e) => e.stopPropagation()} onPointerDown={(e) => e.stopPropagation()}>
                      <svg viewBox="0 0 24 24" className="h-4 w-4" fill="currentColor">
                        <circle cx="5" cy="12" r="1.7" />
                        <circle cx="12" cy="12" r="1.7" />
                        <circle cx="19" cy="12" r="1.7" />
                      </svg>
                    </button>
                  </DropdownMenu.Trigger>
                  <DropdownMenu.Portal>
                    <DropdownMenu.Content className="z-50 min-w-[220px] rounded-xl border border-ink-900/10 bg-white p-1 shadow-lg" align="center" sideOffset={8}>
                      <DropdownMenu.Item className="flex cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-sm text-ink-700 outline-none hover:bg-ink-900/5" onSelect={() => onDeleteSession(session.id)}>
                        <svg viewBox="0 0 24 24" className="h-4 w-4 text-error/80" fill="none" stroke="currentColor" strokeWidth="1.8">
                          <path d="M4 7h16" /><path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" /><path d="M7 7l1 12a1 1 0 0 0 1 .9h6a1 1 0 0 0 1-.9l1-12" />
                        </svg>
                        Delete this session
                      </DropdownMenu.Item>
                      <DropdownMenu.Item className="flex cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-sm text-ink-700 outline-none hover:bg-ink-900/5" onSelect={() => resetResumeSession(session.id)}>
                        <svg viewBox="0 0 24 24" className="h-4 w-4 text-ink-500" fill="none" stroke="currentColor" strokeWidth="1.8">
                          <path d="M4 5h16v14H4z" /><path d="M7 9h10M7 12h6" /><path d="M13 15l3 2-3 2" />
                        </svg>
                        Resume in Claude Code
                      </DropdownMenu.Item>
                    </DropdownMenu.Content>
                  </DropdownMenu.Portal>
                </DropdownMenu.Root>
              </div>
            )}
          </div>
        ))}
      </div>
      <div className={`mt-auto flex items-center gap-2 rounded-xl border border-ink-900/10 bg-surface px-3 py-2 text-xs text-muted ${collapsed ? "justify-center" : ""}`}>
        <span className={`h-2 w-2 rounded-full ${connected ? "bg-success" : "bg-error"}`} />
        {!collapsed && <span>{connected ? "Backend connected" : "Backend offline"}</span>}
      </div>
      <Dialog.Root open={!!resumeSessionId} onOpenChange={(open) => { if (!open) resetResumeSession(null); }}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-50 bg-ink-900/40 backdrop-blur-sm" />
          <Dialog.Content className="fixed left-1/2 top-1/2 z-[60] w-full max-w-xl -translate-x-1/2 -translate-y-1/2 rounded-2xl bg-white p-6 shadow-xl">
            <div className="flex items-start justify-between gap-4">
              <Dialog.Title className="text-lg font-semibold text-ink-800">Resume</Dialog.Title>
              <Dialog.Close asChild>
                <button className="rounded-full p-1 text-ink-500 hover:bg-ink-900/10" aria-label="Close dialog">
                  <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M6 6l12 12M18 6l-12 12" />
                  </svg>
                </button>
              </Dialog.Close>
            </div>
            <div className="mt-4 flex items-center gap-2 rounded-xl border border-ink-900/10 bg-surface px-3 py-2 font-mono text-xs text-ink-700">
              <span className="flex-1 break-all">{resumeSessionId ? `claude --resume ${resumeSessionId}` : ""}</span>
              <button className="rounded-lg p-1.5 text-ink-600 hover:bg-ink-900/10" onClick={handleCopyCommand} aria-label="Copy resume command">
                {copied ? (
                  <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2"><path d="M5 12l4 4L19 6" /></svg>
                ) : (
                  <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="1.8"><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15V5a2 2 0 0 1 2-2h10" /></svg>
                )}
              </button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </aside>
  );
}
