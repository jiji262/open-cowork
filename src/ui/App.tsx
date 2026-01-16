import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { PermissionResult } from "@anthropic-ai/claude-agent-sdk";
import { useIPC } from "./hooks/useIPC";
import { useAppStore } from "./store/useAppStore";
import type { ServerEvent } from "./types";
import { Sidebar } from "./components/Sidebar";
import { StartSessionModal } from "./components/StartSessionModal";
import { PromptInput, usePromptActions } from "./components/PromptInput";
import { MessageCard } from "./components/EventCard";
import MDContent from "./render/markdown";

type StreamEventDelta = { type?: string; [key: string]: unknown };
type StreamEventPayload = { type?: string; delta?: StreamEventDelta };
type StreamEventMessage = { type: "stream_event"; event?: StreamEventPayload };

function App() {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const partialMessageRef = useRef("");
  const [partialMessage, setPartialMessage] = useState("");
  const [showPartialMessage, setShowPartialMessage] = useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const sessions = useAppStore((s) => s.sessions);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const showStartModal = useAppStore((s) => s.showStartModal);
  const setShowStartModal = useAppStore((s) => s.setShowStartModal);
  const globalError = useAppStore((s) => s.globalError);
  const setGlobalError = useAppStore((s) => s.setGlobalError);
  const historyRequested = useAppStore((s) => s.historyRequested);
  const markHistoryRequested = useAppStore((s) => s.markHistoryRequested);
  const resolvePermissionRequest = useAppStore((s) => s.resolvePermissionRequest);
  const handleServerEvent = useAppStore((s) => s.handleServerEvent);
  const prompt = useAppStore((s) => s.prompt);
  const setPrompt = useAppStore((s) => s.setPrompt);
  const cwd = useAppStore((s) => s.cwd);
  const setCwd = useAppStore((s) => s.setCwd);
  const pendingStart = useAppStore((s) => s.pendingStart);

  const isStreamEventMessage = (message: unknown): message is StreamEventMessage =>
    typeof message === "object" &&
    message !== null &&
    "type" in message &&
    (message as { type?: unknown }).type === "stream_event";

  const getPartialMessageContent = (eventMessage: StreamEventPayload | undefined) => {
    if (!eventMessage?.delta || typeof eventMessage.delta.type !== "string") return "";
    const realType = eventMessage.delta.type.split("_")[0];
    const value = (eventMessage.delta as Record<string, unknown>)[realType];
    return typeof value === "string" ? value : "";
  };

  const handlePartialMessages = useCallback((partialEvent: ServerEvent) => {
    if (partialEvent.type !== "stream.message") return;

    const message = partialEvent.payload.message;
    if (!isStreamEventMessage(message)) return;

    const eventType = message.event?.type;
    if (eventType === "content_block_start") {
      partialMessageRef.current = "";
      setPartialMessage(partialMessageRef.current);
      setShowPartialMessage(true);
    }

    if (eventType === "content_block_delta") {
      partialMessageRef.current += getPartialMessageContent(message.event) || "";
      setPartialMessage(partialMessageRef.current);
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }

    if (eventType === "content_block_stop") {
      setShowPartialMessage(false);
      setTimeout(() => {
        partialMessageRef.current = "";
        setPartialMessage(partialMessageRef.current);
      }, 500);
    }
  }, []);

  const onEvent = useCallback((event: ServerEvent) => {
    handleServerEvent(event);
    handlePartialMessages(event);
  }, [handleServerEvent, handlePartialMessages]);

  const { connected, sendEvent } = useIPC(onEvent);
  const { handleStartFromModal } = usePromptActions(sendEvent, connected);

  const activeSession = activeSessionId ? sessions[activeSessionId] : undefined;
  const messages = activeSession?.messages ?? [];
  const permissionRequests = activeSession?.permissionRequests ?? [];
  const isRunning = activeSession?.status === "running";

  useEffect(() => {
    if (connected) sendEvent({ type: "session.list" });
  }, [connected, sendEvent]);

  useEffect(() => {
    if (!activeSessionId || !connected) return;
    const session = sessions[activeSessionId];
    if (session && !session.hydrated && !historyRequested.has(activeSessionId)) {
      markHistoryRequested(activeSessionId);
      sendEvent({ type: "session.history", payload: { sessionId: activeSessionId } });
    }
  }, [activeSessionId, connected, sessions, historyRequested, markHistoryRequested, sendEvent]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, partialMessage]);

  const handleNewSession = useCallback(() => {
    useAppStore.getState().setActiveSessionId(null);
    setShowStartModal(true);
  }, [setShowStartModal]);

  const handleDeleteSession = useCallback((sessionId: string) => {
    sendEvent({ type: "session.delete", payload: { sessionId } });
  }, [sendEvent]);

  const handlePermissionResult = useCallback((toolUseId: string, result: PermissionResult) => {
    if (!activeSessionId) return;
    sendEvent({ type: "permission.response", payload: { sessionId: activeSessionId, toolUseId, result } });
    resolvePermissionRequest(activeSessionId, toolUseId);
  }, [activeSessionId, sendEvent, resolvePermissionRequest]);

  const sidebarWidth = sidebarCollapsed ? 0 : 280;
  const sessionStatusLabel = useMemo(() => {
    switch (activeSession?.status) {
      case "running":
        return { label: "Running", tone: "bg-info/10 text-info" };
      case "completed":
        return { label: "Completed", tone: "bg-success/10 text-success" };
      case "error":
        return { label: "Error", tone: "bg-error/10 text-error" };
      default:
        return { label: "Idle", tone: "bg-ink-900/5 text-ink-600" };
    }
  }, [activeSession?.status]);

  return (
    <div
      className="app-shell relative flex h-screen overflow-hidden bg-surface"
      style={{ "--sidebar-width": `${sidebarWidth}px` } as React.CSSProperties}
    >
      {!sidebarCollapsed && (
        <Sidebar
          connected={connected}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
          collapsed={sidebarCollapsed}
          onToggleCollapse={() => setSidebarCollapsed((prev) => !prev)}
        />
      )}

      <main className="relative z-10 flex min-w-0 flex-1 flex-col">
        <div
          className="flex h-12 items-center justify-between border-b border-ink-900/10 bg-surface/80 px-6 text-sm text-ink-700 backdrop-blur select-none"
          style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
        >
          <div className="flex items-center gap-3">
            {sidebarCollapsed && (
              <button
                type="button"
                className="flex h-8 w-8 items-center justify-center rounded-full border border-ink-900/10 bg-surface text-ink-600 hover:bg-surface-tertiary hover:text-ink-800 transition-colors"
                onClick={() => setSidebarCollapsed(false)}
                aria-label="Expand sidebar"
                style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
              >
                <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M9 6l6 6-6 6" />
                </svg>
              </button>
            )}
            <span className="font-semibold text-ink-800">{activeSession?.title || "open-cowork"}</span>
            {activeSession && (
              <span className={`rounded-full px-2.5 py-0.5 text-[11px] font-semibold uppercase tracking-[0.12em] ${sessionStatusLabel.tone}`}>
                {sessionStatusLabel.label}
              </span>
            )}
          </div>
          <div className="flex items-center gap-2 text-xs">
            <span className={`h-2 w-2 rounded-full ${connected ? "bg-success" : "bg-error"}`} />
            <span className="text-muted">{connected ? "Connected" : "Offline"}</span>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto px-4 pb-56 pt-6 lg:px-8">
          <div className="mx-auto max-w-4xl">
            {activeSession && (
              <div className="mb-6 rounded-2xl border border-ink-900/10 bg-panel/80 p-5 shadow-soft backdrop-blur">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div className="flex flex-col gap-1">
                    <span className="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-light">Session</span>
                    <span className="text-base font-semibold text-ink-800">{activeSession.title}</span>
                    <span className="text-xs text-muted">{activeSession.cwd ? `Working dir: ${activeSession.cwd}` : "Working dir not set"}</span>
                  </div>
                  <span className={`rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.12em] ${sessionStatusLabel.tone}`}>
                    {sessionStatusLabel.label}
                  </span>
                </div>
              </div>
            )}
            {messages.length === 0 ? (
              <div className="flex flex-col items-center justify-center rounded-3xl border border-ink-900/10 bg-panel/80 px-6 py-16 text-center shadow-soft backdrop-blur">
                <div className="text-lg font-semibold text-ink-800">Ready for your next task</div>
                <p className="mt-2 text-sm text-muted">Describe a goal and let open-cowork orchestrate the steps.</p>
                <button
                  className="mt-5 rounded-full bg-accent px-5 py-2 text-sm font-semibold text-white shadow-soft hover:bg-accent-hover transition-colors"
                  onClick={handleNewSession}
                  style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
                >
                  + New Task
                </button>
              </div>
            ) : (
              <div className="message-stack">
                {messages.map((msg, idx) => (
                  <div key={idx} className="message-card">
                    <MessageCard
                      message={msg}
                      isLast={idx === messages.length - 1}
                      isRunning={isRunning}
                      permissionRequest={permissionRequests[0]}
                      onPermissionResult={handlePermissionResult}
                    />
                  </div>
                ))}
              </div>
            )}

            <div className="partial-message mt-6">
              <MDContent text={partialMessage} />
              {showPartialMessage && (
                <div className="mt-3 flex flex-col gap-2 px-1">
                  <div className="relative h-3 w-2/12 overflow-hidden rounded-full bg-ink-900/10">
                    <div className="absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-ink-900/30 to-transparent animate-shimmer" />
                  </div>
                  <div className="relative h-3 w-full overflow-hidden rounded-full bg-ink-900/10">
                    <div className="absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-ink-900/30 to-transparent animate-shimmer" />
                  </div>
                  <div className="relative h-3 w-full overflow-hidden rounded-full bg-ink-900/10">
                    <div className="absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-ink-900/30 to-transparent animate-shimmer" />
                  </div>
                  <div className="relative h-3 w-full overflow-hidden rounded-full bg-ink-900/10">
                    <div className="absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-ink-900/30 to-transparent animate-shimmer" />
                  </div>
                  <div className="relative h-3 w-4/12 overflow-hidden rounded-full bg-ink-900/10">
                    <div className="absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-ink-900/30 to-transparent animate-shimmer" />
                  </div>
                </div>
              )}
            </div>

            <div ref={messagesEndRef} />
          </div>
        </div>

        <PromptInput sendEvent={sendEvent} connected={connected} />
      </main>

      {showStartModal && (
        <StartSessionModal
          cwd={cwd}
          prompt={prompt}
          pendingStart={pendingStart}
          onCwdChange={setCwd}
          onPromptChange={setPrompt}
          onStart={handleStartFromModal}
          onClose={() => setShowStartModal(false)}
        />
      )}

      {globalError && (
        <div className="fixed bottom-24 left-1/2 z-50 -translate-x-1/2 rounded-xl border border-error/20 bg-error-light px-4 py-3 shadow-lg">
          <div className="flex items-center gap-3">
            <span className="text-sm text-error">{globalError}</span>
            <button className="text-error hover:text-error/80" onClick={() => setGlobalError(null)}>
              <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth="2"><path d="M18 6L6 18M6 6l12 12" /></svg>
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
