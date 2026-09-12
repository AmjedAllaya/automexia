if [[ $AMX_TEST_CASE == function ]]; then function amx { print -r -- AMX_EXISTING_OK; }; fi
if [[ $AMX_TEST_CASE == alias ]]; then alias amx='print -r -- AMX_EXISTING_OK'; fi
if [[ $AMX_TEST_CASE == disabled ]]; then export AUTOMEXIA_AMX=0; fi
source "$AMX_TEST_SOURCE" >/dev/null 2>/dev/null
source "$AMX_TEST_SOURCE" >/dev/null 2>/dev/null
print -r -- AMX_OUTPUT_BEGIN
case $AMX_TEST_CASE in
  disabled) (( ${+functions[amx]} )) && exit 3; print -r -- AMX_DISABLED_OK ;;
  function|alias|external) amx ;;
  local) amx find file Dockerfile ;;
  searches)
    for provider in google github youtube ddg; do
      amx search "$provider" --print-url 'fixture & query' || exit 4
    done
    amx docs kubernetes --print-url 'fixture & query' ;;
  missing) AUTOMEXIA_CLI=missing-fixture-executable amx google fixture 2>/dev/null; test $? -eq 127 && print -r -- AMX_MISSING_OK ;;
  *) amx google --print-url 'café & rust' '+#%' 'two words' ;;
esac
