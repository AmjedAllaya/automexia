Fixed QA diagnostic redaction for case-folded and escaped workspace/home
prefixes. Literal-prefix matching keeps workspace labels specific, preserves
unrelated text and leaves original failed exits unchanged. Fictional fixtures
and a real failed-subprocess regression cover the disclosure path; output
limits and source-identity gates are unchanged. This diagnostic correction
does not resolve the separately recorded intermittent native CMD resize failure.
