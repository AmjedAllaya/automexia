Fixed

- Context contribution decoding rejects excess status entries before decoding
  their payloads, preserving the existing 64-entry wire contract. Added streaming
  regressions, guarded benchmark dispatch and checked decode/drop measurements.
