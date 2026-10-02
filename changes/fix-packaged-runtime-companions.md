Include the synchronous `amx` launcher and suggestion helper alongside Automexia
in every installer, portable archive, and macOS universal app. Preserve all
runtime executables through release staging, signing, and final verification.
Fail before replacing portable staging when a required input is missing, and
prevent cargo-packager from rebuilding supplied signed executables.
Attach the existing Windows product/version resources to the standalone `amx`
launcher, which does not link the application library that carries them.

Add archive-content and release-pipeline regressions, expand native package
smoke checks to companion installation and removal, and synchronize the fuzz
lockfile with the workspace's existing dependency requirements.
