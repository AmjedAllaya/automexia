Operational status colours now distinguish completed/stopped workloads from
ready services. Kubernetes `0/1 Completed` and clean container exits use cyan;
fully ready/healthy resources use green, transitional states amber, and explicit
failures red. Status fields take precedence over incidental resource names.

Condition polarity, init failures, container health/paused states and mixed
failure summaries have regression coverage. Literal output, application ANSI,
selection/copy and configured colours remain unchanged. No new dependency or
background work is introduced. Native desktop visual assurance remains separate
from the controlled parser/grid/CPU tests.
