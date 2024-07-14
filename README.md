# Marskman
A CLI written in Rust to snipe reservations!

```
$ marksman --help
Snipe reservations in NYC

Usage: marksman [COMMAND]

Commands:
  venue  Details about venue
  load   Load auth credentials for Resy API
  state  current marksman configuration
  snipe  configure sniper for the reservation
  setup  configure setup wizard
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### TODO 

- [X] Ingest target restaurant (via URL)
- [X] Persistent config (.marksman.config)
- [X] Fetch open reservations for a date (w/ table view)
- [X] Schedule sniper to acquire reservation
- [ ] Background running sniper 
- [ ] Beautiful CLI UI to input target
- [ ] Search functionality
