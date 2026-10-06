# Phase 2 fixture matrix

The suite separates **Raw GPS truth**, **processing/matcher fixtures**, and **publication/worker fixtures**.
The master dataset is staged so the same historical day can be reprocessed as new Raw GPS arrives.

| Decision/problem | Primary fixture | Expected assertion |
|---|---|---|
| Q1 Trip vs MovementSegment | `master/stage-01-base` | 06:40-07:10 is one Trip with WALK then CAR segments |
| Q2 Raw GPS Gap | `master/stage-01-base` | 17:45-17:55 is Gap; no connector/distance |
| Q3 Cross-midnight activity | Stage 2 + `master/oracle.json` | 19:00-00:20 becomes one Stop spanning two Daily Views |
| Q4 Late data publication | Stage 1 -> Stage 2 | old snapshot remains until atomic replacement |
| Q5 UNKNOWN mode | `matcher-fixtures/09-unknown-mode.json` | no OSRM call; raw fallback |
| Q6 Matcher determinism/evidence | matcher fixtures 01-08 | normalized evidence determines derived result |
| Q7 30k scale | `scale-30000/` | exactly 30,000 valid Raw records |
| Q8 Stable range/open edge | master first/last activities | left/right activities are OPEN at dataset edges |
| Q9 Multiday atomic publication | Stage 2 + worker fixture | both affected days publish together |
| Q10 Timezone reprojection | `projection-fixtures/timezone-change.json` | activity UTC unchanged; projections rebuilt |
| Q11 Poor evidence valid snapshot | evidence hole + sparse fixture | ready snapshot may have null route/empty timeline |
| Q12 Mode hysteresis | 06:40-07:10 + mode fixture | no per-second segment oscillation |
| Q13 Route parts/anchors | matcher good/partial fixtures | playback contract is parts + anchors |
| Q14 Evidence retention/retry | fixtures 07-08 | finite retry; evidence preserved |
| Q15 Web polling revision | `worker-fixtures/status-sequence.json` | full refetch only when published revision changes |
| Q16 Low quality != Gap | 12:05-12:10 master | Evidence Hole, not Gap |
| Q17 observed boundaries | master first/last activities | no extrapolation to now/day end |
| Q18 Revision-scoped IDs | `publication-fixtures/revision-id-reset.json` | new revision resets selection/playback |
| Q19 input_generation | stages 1-4 | new committed Batch increments; replay/conflict do not |
| Q20 summary/evidence | `master/oracle.json` | trip/stop/gap and usable/raw counts kept distinct |
| Q21 invalid anchors | matcher fixture 04 | reject pretty matched geometry, fallback |
| Q22 range revision storage | `publication-fixtures/range-replacement.json` | unchanged history reused |
| Q23 worker fencing | `worker-fixtures/publication-races.json` | stale generation/target/token cannot publish |
| Q24 Evidence Hole OPEN boundaries | 12:05-12:10 master | activities on both sides have OPEN adjacent boundaries |
| Q25 distance conservation | `progress-fixtures/01-cross-midnight-distance.json` | adjacent day distances sum to source progress delta |
| Q26 same-second reduction | Stage 3 + matcher fixture 10 | deterministic winner; no timestamp rewriting |
| Q27 chunk seam | matcher fixtures 05-06 | seam by shared source/time/progress, else fallback |
