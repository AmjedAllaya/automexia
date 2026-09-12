if test "$AMX_TEST_CASE" = function; or test "$AMX_TEST_CASE" = alias
    function amx; printf '%s\n' AMX_EXISTING_OK; end
end
if test "$AMX_TEST_CASE" = disabled; set -gx AUTOMEXIA_AMX 0; end
source "$AMX_TEST_SOURCE" >/dev/null 2>/dev/null
source "$AMX_TEST_SOURCE" >/dev/null 2>/dev/null
printf '%s\n' AMX_OUTPUT_BEGIN
switch "$AMX_TEST_CASE"
case disabled
    functions -q amx; and exit 3
    printf '%s\n' AMX_DISABLED_OK
case function alias external
    amx
case local
    amx find file Dockerfile
case missing
    set -gx AUTOMEXIA_CLI missing-fixture-executable
    amx google fixture 2>/dev/null
    test $status -eq 127; and printf '%s\n' AMX_MISSING_OK
case searches
    for provider in google github youtube ddg
        amx search "$provider" --print-url 'fixture & query'; or exit 4
    end
    amx docs kubernetes --print-url 'fixture & query'
case '*'
    amx google --print-url 'café & rust' '+#%' 'two words'
end
