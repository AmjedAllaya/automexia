if [[ $AMX_TEST_CASE == function ]]; then amx() { printf '%s\n' AMX_EXISTING_OK; }; fi
if [[ $AMX_TEST_CASE == alias ]]; then
  shopt -s expand_aliases
  alias amx='printf "%s\n" AMX_EXISTING_OK'
fi
if [[ $AMX_TEST_CASE == disabled ]]; then export AUTOMEXIA_AMX=0; fi
# The native harness supplies and executes the real adapter; lint each adapter
# separately instead of following this runtime-selected source.
# shellcheck source=/dev/null
source "$AMX_TEST_SOURCE" >/dev/null 2>/dev/null
# shellcheck source=/dev/null
source "$AMX_TEST_SOURCE" >/dev/null 2>/dev/null
printf '%s\n' AMX_OUTPUT_BEGIN
case $AMX_TEST_CASE in
  disabled) declare -F amx >/dev/null && exit 3; printf '%s\n' AMX_DISABLED_OK ;;
  function|alias|external) amx ;;
  local) amx find file Dockerfile ;;
  searches)
    for provider in google github youtube ddg; do
      amx search "$provider" --print-url 'fixture & query' || exit 4
    done
    amx docs kubernetes --print-url 'fixture & query' ;;
  missing) AUTOMEXIA_CLI=missing-fixture-executable amx google fixture 2>/dev/null; test $? -eq 127 && printf '%s\n' AMX_MISSING_OK ;;
  *) amx google --print-url 'café & rust' '+#%' 'two words' ;;
esac
