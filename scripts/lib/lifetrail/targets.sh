# Named application groups and their optional Compose profiles.
select_apps() {
  COMPOSE_PROFILE_ARGS=()
  case "${1:-all}" in
    all)
      COMPOSE_PROFILE_ARGS=(--profile osrm)
      SERVICES=(web server postgres osrm-car osrm-bike osrm-foot)
      ;;
    osrm)
      COMPOSE_PROFILE_ARGS=(--profile osrm)
      SERVICES=(osrm-car osrm-bike osrm-foot)
      ;;
    osrm-car|osrm-bike|osrm-foot)
      COMPOSE_PROFILE_ARGS=(--profile osrm)
      SERVICES=("$1")
      ;;
    web|server|postgres) SERVICES=("$1") ;;
    *)
      printf 'Unknown app: %s\n' "$1" >&2
      return 2
      ;;
  esac
}
