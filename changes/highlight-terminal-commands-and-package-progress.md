Changed

- Integrated command input uses different theme colors for command names, flags,
  arguments and shell syntax. Multiline commands keep their colors after
  submission when the shell marks them as input.
- APT downloads, transfer summaries, dpkg progress, and tzdata reports use
  distinct text colors for phases, package names, versions, and measurements.
  Repository URLs and previous versions use a subtler accent. Terminal output
  colors controls this highlighting in local, WSL and remote sessions. The
  renderer preserves application ANSI styles and the original terminal text.
