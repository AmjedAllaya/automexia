Bind local QA evidence to changed source contents rather than a file-status
list. Fail full QA when its source identity changes or cannot be established;
bound inventory collection, file hashing and subprocess lifetime. Add
content-mutation and actual QA-report regression tests.
Keep report and optional bundle announcements repository-relative; cover both
successful and failed CLI runs without exposing an absolute checkout location.
