import type { Map } from "maplibre-gl";

/** Keep navigation pitch stable, then restore ordinary map controls on exit. */
export class PlaybackInteraction {
  private restore: (() => void) | undefined;

  constructor(private readonly map: Map) {}

  setFollowing(active: boolean): void {
    if (!active) {
      this.restore?.();
      this.restore = undefined;
      return;
    }
    if (this.restore) return;
    const minPitch = this.map.getMinPitch();
    const maxPitch = this.map.getMaxPitch();
    const handlers = [this.map.dragRotate, this.map.touchPitch];
    const enabled = handlers.map((handler) => handler.isEnabled());
    handlers.forEach((handler) => handler.disable());
    // Pitch bounds also cover keyboard, touch rotation and compass gestures.
    this.map.setMaxPitch(60);
    this.map.setMinPitch(55);
    this.map.setMaxPitch(55);
    this.restore = () => {
      this.map.setMinPitch(0);
      this.map.setMaxPitch(maxPitch);
      this.map.setMinPitch(minPitch);
      handlers.forEach((handler, index) => {
        if (enabled[index]) handler.enable();
      });
    };
  }

  dispose(): void {
    this.setFollowing(false);
  }
}
