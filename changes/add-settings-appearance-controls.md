# Font size and adaptive appearance in Settings

Settings now exposes the existing saved font size and adaptive appearance choices.
Font controls preserve fractional sizes within the 6–100 point runtime range.
Reset inherits current configuration. Appearance uses loaded Light/Dark palettes;
fixed palettes explain unavailability and retain any saved choice.

Live updates reuse loaded resources and update inactive local tabs without
reloading background images. Existing shortcuts and Settings use the same
application preference owner and asynchronous writer. No storage migration is
needed. Named-theme discovery, general palette editing and native accessibility
remain separate from these controls.
