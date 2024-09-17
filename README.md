# PORTx

A terminal user interface (TUI) with touches of infographics.

The main output should be a grid with 65536 (256²) elements representing all the ports in a machine. Visually, the user should be able to see the ports that are listening - or with an established connection - and the protocol (TCP or UDP). 

Without the use of any alphanumeric characters, the user should get enough useful information. 

The most probable visual technique would be a heatmap.


## Testing 

### with ss `command`

```bash
$ sudo ss -tunlp

-t: display tcp sockets
-u: display udp sockets
-n: show exact bandwidth values, instead of human-readable
-l: display only listening sockets
-p: show process using socket
```

## Code Quality

- Formatter:
```bash
$ cargo fmt
```

- Style and performance
```bash
$ cargo clippy
```

## Documentation

[Ratatui](https://docs.rs/ratatui/latest/ratatui/index.html)

