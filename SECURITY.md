# Security policy

## Supported version

Security fixes are applied to the latest published release.

## Reporting a vulnerability

Do not open a public issue containing credentials, tokens, private network details, or a working exploit. Contact the repository owner privately through the security-reporting method configured on GitHub.

## Deployment guidance

- Restrict Homebridge Config UI and port `8581` to a trusted LAN.
- Do not expose Homebridge insecure mode directly to the internet.
- Protect the Linux user account and OpenDeck settings directory.
- Redact passwords, tokens, public hostnames, and sensitive IP addresses from logs.
