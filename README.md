# xdcc-cli

CLI to download files via XDCC.

```bash
cargo r --release -- irc.artikanet.org:6667 'GX|BDRip|01' 11 --channels GALAXY --request-timeout-secs 100

# You can set RUST_LOG=debug to show debug messages.
RUST_LOG=debug cargo r --release -- irc.artikanet.org:6667 'GX|BDRip|01' 11 --channels GALAXY --request-timeout-secs 100
```

## Disclaimer

When downloading files, users are subject to country-specific software
distribution laws. xdcc-cli is not designed to enable illegal activity. We do
not promote piracy nor do we allow it under any circumstances. You should own
an original copy of every content downloaded through this tool. Please take
the time to review copyright and video distribution laws and/or policies for
your country before proceeding.

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
