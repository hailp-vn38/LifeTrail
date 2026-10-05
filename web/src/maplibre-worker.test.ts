import { describe, expect, it, vi } from "vitest";

const setWorkerUrl = vi.fn();

vi.mock("maplibre-gl", () => ({ setWorkerUrl }));

describe("configureMapLibreWorker", () => {
  it("sets a Vite-emitted worker URL before a map is created", async () => {
    const { configureMapLibreWorker } = await import("./maplibre-worker");

    configureMapLibreWorker();

    expect(setWorkerUrl).toHaveBeenCalledWith(expect.stringContaining("worker"));
  });
});
