/**
 * Timeline event model.
 *
 * Phase 1 only produces `start`/`end`. The kind union is intentionally
 * wider so Phase 2 (trip, stop, photo, audio) extends the timeline without
 * changing the component contracts.
 */
export type TimelineEventKind = "start" | "end" | "trip" | "stop" | "photo" | "audio";

export interface TimelineEvent {
  id: string;
  kind: TimelineEventKind;
  title: string;
  subtitle?: string;
  /** [longitude, latitude] for map flyTo on selection. */
  coordinate?: [number, number];
  /** GPS epoch ms, when known. */
  recordedAtMs?: number;
}
