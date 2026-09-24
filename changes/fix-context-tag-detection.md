Fix passive context tags to use the active shell's bounded location and user
metadata. Native Windows PowerShell publishes Kubernetes home candidates without
filesystem checks on the prompt path; guest sessions cannot select host provider
context as a fallback. Preserve bounded configuration parsing and explicit opt-out,
and test candidate changes, clearing, invalid input, and cached native frames.
