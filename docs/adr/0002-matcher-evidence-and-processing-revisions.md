# Fixed matcher evidence and explicit processing revisions

Phase 2 alignment: superseded by [ADR-0007](0007-phase2-processed-gps.md). The text below records the earlier decision.


Accepted on 2026-10-06. Deterministic processing means that fixed Raw GPS, processing version, configuration, matcher input and matcher evidence produce the same normalized derived result. It does not require an online OSRM run and an outage run to produce identical geometry. Match operations retain request/input hash, profile, dataset and engine versions, status, confidence and normalized result/evidence; outage evidence explains fallback decisions.

A published raw-fallback result is a valid immutable processing revision. OSRM recovery may upgrade it only through explicit reprocessing and a new publication, rather than silently changing a previously published snapshot. This requires evidence storage and revision management but makes historical outputs explainable and reproducible.

Round 2 confirmed on 2026-10-06: retain complete matcher inputs, request parameters, normalized results or failures, attempt timing and provenance without automatic garbage collection in Phase 2. Retry only transient dependency failures within finite attempt/time budgets; NoMatch, valid low confidence and invalid inputs go directly to processing policy. Published fallback does not automatically upgrade when OSRM recovers.

Only sufficiently confident WALK/BIKE/CAR classifications select foot/bike/car profiles. UNKNOWN and below-threshold candidates use raw fallback. Trying all profiles and choosing the best-looking match was rejected because successful road matching does not prove transport mode and can misrepresent rail or water travel.
