# Self-hosted runner requirements

Release/assurance runners are part of the trusted computing base.
Production runners should be ephemeral or reimaged after every job,
assigned to a restricted runner group, unavailable to forked pull
requests, fully patched, monitored, and used only by protected
workflows. Remove residual workspaces, credentials, signing material,
caches containing secrets, mounted shares, and user profiles after each
run. Restrict outbound network destinations and interactive logon, and
maintain an auditable software/firmware inventory.
