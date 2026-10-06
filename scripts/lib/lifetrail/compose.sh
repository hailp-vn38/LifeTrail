# Compose invocation shared by lifecycle and one-shot commands.
compose() {
  local -a env_file_args=()

  if [[ -f "$WEB_ENV_FILE" ]]; then
    env_file_args=(--env-file "$WEB_ENV_FILE")
  fi

  docker compose "${env_file_args[@]}" -f "$COMPOSE_FILE" "$@"
}
