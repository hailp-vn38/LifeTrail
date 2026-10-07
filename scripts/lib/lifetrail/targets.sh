# Named application groups and their optional Compose profiles.
select_apps() {
  COMPOSE_PROFILE_ARGS=()
  SERVICES=()
  if [[ $# -eq 0 || ( $# -eq 1 && "$1" == all ) ]]; then
    SERVICES=(web server postgres)
    return 0
  fi

  local app
  for app in "$@"; do
    case "$app" in
      web|server|postgres) SERVICES+=("$app") ;;
      *)
        printf 'Unknown app: %s (use all on its own)\n' "$app" >&2
        return 2
        ;;
    esac
  done
}
