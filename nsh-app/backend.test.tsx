// The Rust ↔ TSX seam: the generated declaration is callable from a test and
// reports the schema the transport speaks.
import { expect, test } from "nsh/test";
import { Backend } from "aalc/native";

test("the Rust backend exposes the contract schema and its mode", () => {
  expect(Backend.schemaVersion()).toBe(3);
  expect(["mock", "sidecar", "unavailable"]).toContain(Backend.mode());
});

test("the mock transport answers app.ping through the Rust boundary", async () => {
  // The normal suite runs against the configured transport; only the mock is
  // started on purpose here (run with AHAB_BACKEND=mock).
  if (Backend.mode() !== "mock") return;
  expect(await Backend.ping()).toBe("pong");
});
