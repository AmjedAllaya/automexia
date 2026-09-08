Completed output highlights now use subtle separated row bands, improving table
readability for Kubernetes and other commands. Text, alignment, copying and
selection remain on the original terminal grid; no blank output lines are added.
The existing completion pulse, reduced-motion behavior and prompt gutter remain.
Regression coverage includes parser-created tables, resize/scroll projection,
retained copy bytes, bounded geometry, exact gap coverage and renderer benchmarks.
