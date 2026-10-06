# Named application groups and their optional Compose profiles.
select_apps() {
  COMPOSE_PROFILE_ARGS=()
  case "${1:-all}" in
    all) SERVICES=(web server postgres) ;;
    web|server|postgres) SERVICES=("$1") ;;
    *)
      printf 'Unknown app: %s\n' "$1" >&2
      return 2
      ;;
  esac
}
