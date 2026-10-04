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
A read model for one Owner-local calendar day; its time boundaries are resolved from that Owner's IANA timezone while underlying records remain UTC.
_Avoid_: UTC day, device day

**Route**:
A derived GeoJSON LineString representing the GPS history in a Daily View. It is not the Raw GPS source of truth.
_Avoid_: GPS log, track file

**Quarantine**:
Durable local storage for a Batch whose integrity cannot be safely established and which must never be deleted automatically.
_Avoid_: trash, failed upload
