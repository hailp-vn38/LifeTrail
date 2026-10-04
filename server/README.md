# Rust server

This modular-monolith crate owns provisioning, device ingestion, persisted Raw GPS and Daily View APIs. It must implement the contracts in [`../protocol/`](../protocol/README.md), not redefine them.

The executable is deliberately inert until the server-foundation ticket.
