usage() {
  cat <<'HELP'
Usage: scripts/lifetrail <command> [app]
       scripts/lifetrail simulate [simulator arguments...]

Apps:
  all       web, server, postgres, osrm-car, osrm-bike, osrm-foot (default).
  osrm      All three OSRM services.
  web | server | postgres | osrm-car | osrm-bike | osrm-foot

Commands:
  start [app]          Build if needed and start the selected services.
  stop [app]           Stop the selected services without removing data.
  restart [app]        Restart the selected services.
  status [app]         Show running and stopped selected services.
  logs [app]           Follow the selected services' logs.
  build [all|web|server]
                      Build application images (OSRM uses a pinned image).
  test                Check Web and API through Nginx at :8080.
  test server-tests   Run all server tests, including PostGIS integration,
                      serially in Docker with a dedicated test database.
  simulate [args...]   Run the one-shot OSRM simulator in Docker.

Examples:
  scripts/lifetrail start all
  scripts/lifetrail status all
  scripts/lifetrail logs osrm
  scripts/lifetrail test server-tests
  scripts/lifetrail simulate --help
  LT_OSRM_DATASET_NAME=test-region LT_OSRM_DATASET_VERSION=monaco-test \
  scripts/lifetrail simulate --scenario tools/scenarios/monaco-car.json \
    --metadata data/osrm/monaco-test/car/metadata.json \
    --osrm-url http://osrm-car:5000 --output runtime/osrm-demo/car

Prepare graphs first with tools/prepare_osrm.py. The default is
data/osrm/{car,bike,foot}/vietnam.osrm*. LT_OSRM_DATA_DIR overrides the
root; LT_OSRM_DATASET_VERSION selects a subdirectory and
LT_OSRM_DATASET_NAME selects the graph basename. data/osrm may be a
symlink to storage. OSRM remains internal, with no host ports.
HELP
}
