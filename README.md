
# CouchOS Fire TV Integration

A native CouchOS integration for Amazon Fire TV devices, providing remote control over the local Fire TV REST API.

The integration supports secure PIN-based pairing and credential management through CouchOS, allowing users to control their Fire TV directly from their CouchOS remote.

## Features

- **PIN pairing** — Pair with your Fire TV using a four-digit PIN. Authentication credentials are securely managed by CouchOS.
- **Navigation** — Up, Down, Left, Right, Select, Back, Home, and Menu.
- **Media controls** — Play/Pause, Fast Forward, and Rewind.
- **Power on** — Wake supported Fire TV devices using the DIAL protocol.

## Compatibility

Designed initially for Amazon Fire TV Cube devices.

Fire OS versions and device models may vary in API support. Compatibility with all Fire TV devices is not guaranteed.

### Not Currently Supported

- Volume and mute controls
- Power off
- Device status reporting
- Input enumeration
- App listing and launching
- Device discovery
- Key-hold and repeat behavior

## Security

Communication uses the Fire TV's local HTTPS API and its device-specific authentication credentials.

The Fire TV API uses a self-signed TLS certificate. The integration accommodates this for local device communication. For security, Fire TV devices should be used on a trusted local network.

Authentication tokens are stored using CouchOS's private credential management rather than ordinary integration settings.

## Credits and Acknowledgements

This integration builds on the work of the following developers:

- **[dangerouslaser](https://github.com/dangerouslaser)** — Creator of CouchOS, its documentation, SDK, and the broader CouchOS integration ecosystem.

- **[Mase (Meir Miyara)](https://github.com/mase1981)** — Creator of the original [Fire TV integration for Unfolded Circle Remote Two and Remote 3](https://github.com/mase1981/uc-intg-firetv). This CouchOS integration is a Rust port and adaptation of that original work.

- **[SLC-Josh](https://github.com/SLC-Josh/FireTVRest)** — Original research and documentation of the Fire TV REST API, including authentication and remote-control commands.

Additional technical reference:
- [HMS Fire TV](https://github.com/hms-homelab/hms-firetv)

Special thanks to these developers for their open-source contributions to the home automation community.
