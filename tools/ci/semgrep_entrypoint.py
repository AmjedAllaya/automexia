#!/usr/bin/env python3
"""Relocatable entry point for the pinned Semgrep virtual environment.

The assurance runner supplies Semgrep's pinned compatibility flag. Keeping the
launcher free of cache paths lets a staged virtual environment be published by
atomic rename without retaining its compiler and download staging tree.
"""

from semgrep.console_scripts.entrypoint import main


if __name__ == "__main__":
    raise SystemExit(main())
