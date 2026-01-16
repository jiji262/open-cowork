import { describe, it, expect } from "vitest";
import { useAppStore } from "./useAppStore";
import type { ServerEvent } from "../types";

describe("provider config", () => {
  it("sets active provider", () => {
    useAppStore.getState().setActiveProvider("anthropic");
    expect(useAppStore.getState().activeProvider).toBe("anthropic");
  });
});

describe("stream messages", () => {
  it("ignores stream_event messages for session history", () => {
    const sessionId = "session-stream-event";
    useAppStore.setState({ sessions: {} });

    const statusEvent: ServerEvent = {
      type: "session.status",
      payload: { sessionId, status: "running" }
    };
    useAppStore.getState().handleServerEvent(statusEvent);

    const before = useAppStore.getState().sessions[sessionId]?.messages.length ?? 0;

    const streamEvent: ServerEvent = {
      type: "stream.message",
      payload: {
        sessionId,
        message: { type: "stream_event", event: { type: "content_block_delta" } }
      }
    };
    useAppStore.getState().handleServerEvent(streamEvent);

    const after = useAppStore.getState().sessions[sessionId]?.messages.length ?? 0;
    expect(after).toBe(before);
  });
});
