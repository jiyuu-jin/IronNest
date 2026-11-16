# wifi

A Wi-Fi radio on Linux, spoken to over nl80211 (generic netlink) on a raw
socket, with no `iw`, `wpa_supplicant` or DHCP client:

- scan (`Radio::scan`), join an open network (`join`, with retries for
  drivers that report every failure the same way), leave it;
- set the interface's IPv4 address and link (`set_address`, `set_link`);
- lend the radio's phy to another network namespace (`lend_radio`).

Message building and parsing are pure functions with tests. Joining a
network that needs a key is not supported.
