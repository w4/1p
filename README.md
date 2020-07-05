# 1p-cli

A user-friendly frontend to the [op] command-line tool distributed by
AgileBits giving you a `pass`-like interface into the world of centralised
password management.

```
$ eval $(op signin my) # for my.1password.com
$ 1p ls
Jordan Doyle (my)
├── Guest House Network
│   ├── switch0-3-6
│   └── Wireless Router
└── Personal
    ├── SoundCloud
    ├── Ladbrokes
    ├── Government Gateway
    ├── Le Club AccorHotels
    └── ...
```

This tool is in very early infancy, if you've stumbled upon this project feel
free to use it, however you may run into some very strange looking errors - such
as when your login token expires. If you're comfortable using Rust and diving into
the code feel free to check-in your changes!

[op]: https://1password.com/downloads/command-line/