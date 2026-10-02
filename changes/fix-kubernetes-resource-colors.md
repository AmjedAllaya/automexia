Fixed

- Preserve independent Kubernetes colors for recognized workload, storage,
  namespace, service, event, autoscaler and certificate tables, including
  wrapped rows, empty pending-volume columns and tab-delimited fields.
- Read API-service availability before its diagnostic reason, preserving
  failure and unknown colors for statuses such as `False (MissingEndpoints)`.
- Keep unknown states neutral and avoid claiming health from resource names
  or fields that only describe configuration. Reset table context at prompt,
  blank and unrelated-output boundaries.
- Add raw-output, wrapped-table, native palette-pixel and provider-file
  refresh regressions. Saved settings and kubeconfig files remain untouched.
