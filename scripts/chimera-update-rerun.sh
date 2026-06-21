prepare_update_rerun_args() {
  local -a rerun_args=("$@")
  case "${rerun_args[0]:-}" in
    -start|start)
      rerun_args[0]="-restart"
      ;;
  esac
  printf '%s\n' "${rerun_args[@]}"
}

rerun_after_update() {
  exec "$0" "$@"
}
