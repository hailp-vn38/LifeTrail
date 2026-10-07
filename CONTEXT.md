# LifeTrail

LifeTrail records a person's location history from offline-capable devices and presents it as private daily routes. The server owns persisted data and the web is a read-oriented view of it.

## Language

**Owner**:
The single person who owns the Phase 1 deployment and its Devices. The data model may support more Owners later.
_Avoid_: admin, customer, tenant

**Device**:
A provisioned physical recorder belonging to one Owner and authenticated to the server by its own credential.
_Avoid_: client, tracker, ESP32

**GPS Record**:
One immutable, timestamped location observation collected by a Device, including its acquisition-quality metadata.
_Avoid_: event, route point

**Navigation Epoch**:
One GNSS measurement instant for which a valid RMC record and matching GGA record can be merged into a GPS Record.
_Avoid_: parser message, GPS sample window

**Raw GPS**:
The complete set of accepted GPS Records before server-side quality classification or route derivation.
_Avoid_: cleaned GPS, final route

**Batch**:
An immutable Device-owned collection of GPS Records that is uploaded and acknowledged atomically, identified by a non-reusable UUIDv4.
_Avoid_: request, file

**Daily View**:
A read model for one Owner-local calendar day; its time boundaries are resolved from that Owner's IANA timezone while underlying records remain UTC. It publishes a display-oriented Route Part projection rather than canonical geometry.
_Avoid_: UTC day, device day

**Route**:
A spatial representation of observed movement derived from quality-processed GPS observations in Phase 2. A Route may contain disconnected portions separated by GPS Gaps; it is not the Raw GPS source of truth.
_Avoid_: GPS log, track file

**Trip**:
A maximal continuous chain of movement by a Device, ending at a qualifying Stop or GPS Gap once that boundary is confirmed. A Trip contains one or more ordered Movement Segments, may span transport modes or calendar days, and can have open boundaries at observation edges or adjacent to Evidence Holes.
_Avoid_: Movement Segment, day route

**Movement Segment**:
A portion of a Trip with a relatively homogeneous transport mode: walking, cycling, driving or unknown. A mode change can separate Movement Segments without ending their Trip.
_Avoid_: Trip, GPS Record

**Stop**:
A spatial dwell meeting the configured detection criteria, which ends a Trip. One Stop may continue across calendar days or remain open until further observations establish its boundary; a shorter pause does not itself end a Trip.
_Avoid_: zero-speed point, GPS Gap

**GPS Gap**:
An interval without Raw GPS observations between two known observed boundaries that ends a Trip. It does not establish movement or stationary time, and cannot be inferred outside the first/last observations or merely from quality filtering.
_Avoid_: Stop, inferred movement, upload delay

**Evidence Hole**:
An interval containing Raw GPS observations for which reliable activity or geometry cannot be derived. It can leave adjoining activity boundaries open and is distinct from the absence of observations represented by a GPS Gap.
_Avoid_: GPS Gap, Stop

**Published Daily Snapshot**:
A consistent processed Daily View made available to the Owner. The last successful snapshot remains available while a replacement is prepared or fails.
_Avoid_: activity container, processing job

**Stable Segmentation Boundary**:
A confirmed Stop or GPS Gap transition with sufficient observations on both sides that activity beyond it remains unchanged when the adjacent dirty range is reprocessed.
_Avoid_: fixed context window, midnight boundary

**Route Part**:
A contiguous drawable portion of the derived Route for a Movement Segment. A Movement Segment may have several Route Parts; a separation between them does not by itself establish a GPS Gap. Its canonical geometry and progress are authoritative.
_Avoid_: Movement Segment, GPS Gap
_Avoid_: Movement Segment, GPS Gap

**Display Geometry**:
The simplified, coordinate-rounded Route Part geometry that a Daily View publishes for visualization, derived from canonical Route Part geometry. It is never the source of distance, duration or progress.
_Avoid_: route geometry, canonical geometry

**Effective Tolerance**:
The actual simplification tolerance used to produce a Display Geometry after any deterministic escalation to fit the vertex budget, recorded so the result is reproducible.
_Avoid_: baseline tolerance, max tolerance

**Playback Payload**:
The canonical Route Part geometry and temporal progress served on demand by the Playback API for playback, export and video. It is never simplified, rounded or derived from Display Geometry.
_Avoid_: display geometry, daily payload

**Activity Revision**:
An immutable version of derived Device activity for one continuous processing range. It identifies the origin of its events and Route Parts rather than one calendar day's publication.
_Avoid_: Daily Snapshot, full Device history

**Activity Manifest**:
A versioned composition of Activity Revision ranges defining the authoritative history of a Device. A Published Daily Snapshot refers to a particular manifest rather than whichever history is current later.
_Avoid_: latest revision, Daily Snapshot

**Quarantine**:
Durable local storage for a Batch whose integrity cannot be safely established and which must never be deleted automatically.
_Avoid_: trash, failed upload
