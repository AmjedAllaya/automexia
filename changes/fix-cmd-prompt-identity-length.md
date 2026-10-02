Fixed CMD prompts losing the input marker when identity metadata exceeded CMD's
prompt-format limit. A bounded session-local identity reference preserves the
prompt and restores the correct context after nested shells return, including
long Unicode identities and directory changes.
