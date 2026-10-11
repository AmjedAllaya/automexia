Fixed

- Add Connect… (SSH) to the command palette with recent-first public host search,
  independent filters, and visible system OpenSSH ownership.
- Keep host review bound to its selected identity across navigation and refresh;
  remove hidden Hub section hit targets from the dedicated picker.
- Exclude conditional Match values and Include paths from static inventory.
- Cache normalized catalog sort keys once per projection to avoid repeated
  allocations during search. Add model, controller, native keyboard and bounded
  catalog benchmark coverage.
  Managed connection launch remains protected and unavailable in this build.
