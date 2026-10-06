Inline tables now recognize compact numeric columns alongside wider gutters,
including disk-usage output, while preserving multiword labels and paths.
Source-aligned numbers retain their right alignment; selection, copying and ANSI
colors still refer to the original terminal cells. Panes too narrow for compact
aligned values retain native output instead of splitting the numbers or treating
a data row as a replacement header. Regressions and checked benchmarks cover
mixed spacing, wrapping, resize and bounded large tables.
