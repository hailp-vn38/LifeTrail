import { describe, expect, it, vi } from "vitest";
import { initializeWhenMapLoaded } from "./map-lifecycle";

describe("initializeWhenMapLoaded", () => {
  it("initializes immediately when an inline style has already loaded", () => {
    const initialize = vi.fn();
    const once = vi.fn();

    initializeWhenMapLoaded({ loaded: () => true, once }, initialize);

    expect(initialize).toHaveBeenCalledTimes(1);
    expect(once).not.toHaveBeenCalled();
  });

  it("waits for the load event when the style is still loading", () => {
    const initialize = vi.fn();
    let onLoad: (() => void) | undefined;
    const once = vi.fn((_event: string, listener: () => void) => {
      onLoad = listener;
    });

    initializeWhenMapLoaded({ loaded: () => false, once }, initialize);
    expect(initialize).not.toHaveBeenCalled();
    expect(once).toHaveBeenCalledWith("load", expect.any(Function));

    onLoad?.();
    expect(initialize).toHaveBeenCalledTimes(1);
  });
});
