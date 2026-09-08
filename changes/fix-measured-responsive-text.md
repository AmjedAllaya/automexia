Responsive labels now retain whole graphemes and use the actual drawing font,
style and scale when fitting their visible width. Fitting preserves original
editable values and whitespace, rejects invalid geometry, and bounds source
inspection, retained boundaries and measurement probes. Regression coverage
includes the previously overflowing rounded-font case, partial Unicode context,
exact prepared-font CPU rasters and independent resource ceilings.
Candidate measurements reuse capped storage and short boundary lists stay inline;
literal probe traces and capacity-transition tests preserve the measured result.
