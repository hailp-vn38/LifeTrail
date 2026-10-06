usage() {
  cat <<'HELP'
Usage: scripts/lifetrail <command> [app]

Apps:
  all       web, server, postgres (default).
  web | server | postgres

Commands:
  start [app]          Build if needed and start the selected services.
  stop [app]           Stop the selected services without removing data.
  restart [app]        Restart the selected services.
  status [app]         Show running and stopped selected services.
  logs [app]           Follow the selected services' logs.
  build [all|web|server]
                      Build application images.
  test                Check Web and API through Nginx at :8080.
  test server-tests   Run all server tests, including PostGIS integration,
                      serially in Docker with a dedicated test database.

Examples:
  scripts/lifetrail start all
  scripts/lifetrail status all
  scripts/lifetrail logs server
  scripts/lifetrail test server-tests

Phase 2 processes GPS locally; it requires no routing service.
Generate synthetic Batches with python3 tools/simulate_gps.py --help.
HELP
}
