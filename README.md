# CouchOS Fire TV Integration

A native CouchOS integration for Amazon Fire TV devices, providing remote control over the local Fire TV REST API.

The integration supports secure PIN-based pairing and credential management through CouchOS, allowing users to control their Fire TV directly from their CouchOS remote.

## Installation

### 1. Add the Fire TV package repository

In CouchOS, open **Integrations** and find **Add a custom repository**. Enter these values:

| Field | Value |
| --- | --- |
| Repository ID | `firetv` |
| Name | `Fire TV Integration` |
| Base URL | `https://joemacjr92.github.io/couch-integration-firetv/preview` |
| Public signing key | [firetv-repository-public.pem](keys/firetv-repository-public.pem) |

The public signing key is also available as a [raw PEM file](https://raw.githubusercontent.com/joemacjr92/couch-integration-firetv/main/keys/firetv-repository-public.pem). Only the public key is published here; the private signing key is kept private by the repository maintainer.

### 2. Verify and trust the signing key

1. Paste the contents of the public PEM file into CouchOS.
2. Select **Check public-key fingerprint**.
3. Confirm that the displayed SHA-256 fingerprint matches this value:

   `5ff5846b5d3cc1e404bd5a2ac7640414b91baecf20c2b03d0ada812c15b22a14`

4. If it matches, select **Trust repository**.
5. Select **Refresh packages**, then find and install **Fire TV Integration**.

Always verify the fingerprint before trusting the repository. The fingerprint is provided separately here so it can be compared with the value CouchOS calculates from the pasted public key.

### 3. Pair your Fire TV

After installation, add/configure the Fire TV integration from the CouchOS **Connections** area. Follow the integration's PIN-pairing flow and enter the four-digit PIN shown by your Fire TV.

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

The repository's public signing key is published in this repository for users to verify and trust the package feed. The corresponding private signing key is not included in the repository.

## Credits and Acknowledgements

This integration builds on the work of the following developers:

- **[dangerouslaser](https://github.com/dangerouslaser)** — Creator of CouchOS, its documentation, SDK, and the broader CouchOS integration ecosystem.

- **[Mase (Meir Miyara)](https://github.com/mase1981)** — Creator of the original [Fire TV integration for Unfolded Circle Remote Two and Remote 3](https://github.com/mase1981/uc-intg-firetv). This CouchOS integration is a Rust port and adaptation of that original work.

- **[SLC-Josh](https://github.com/SLC-Josh/FireTVRest)** — Original research and documentation of the Fire TV REST API, including authentication and remote-control commands.

Additional technical reference:
- [HMS Fire TV](https://github.com/hms-homelab/hms-firetv)

Special thanks to these developers for their open-source contributions to the home automation community.
