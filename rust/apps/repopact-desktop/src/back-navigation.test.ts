import { describe, expect, it } from "vitest";
import { resolveBackAction } from "./App";

describe("resolveBackAction (WI060 AND-011)", () => {
  it("closes an open detail view first, even if the drawer is also open", () => {
    expect(resolveBackAction({ detail: { kind: "work", id: "001" }, navOpen: true })).toBe("close-detail");
  });

  it("closes an open detail view when the drawer is closed", () => {
    expect(resolveBackAction({ detail: { kind: "decision", id: "0001" }, navOpen: false })).toBe("close-detail");
  });

  it("closes the compact navigation drawer when no detail is open", () => {
    expect(resolveBackAction({ detail: null, navOpen: true })).toBe("close-nav");
  });

  it("falls through to exit when there is no internal navigation state to unwind", () => {
    expect(resolveBackAction({ detail: null, navOpen: false })).toBe("exit");
  });
});
