# Security

Email contact@maxxit.app with a reproduction, affected version, and impact. Do not publish active credentials in issues. The preview release has no independent security audit.

Provider sign-in is owned by each official CLI. Maxxit never loads their authentication files. Device pairing expires after ten minutes and can be redeemed once. Cloud credentials are stored in Mac Keychain. Local data is retained for 90 days. The renderer cannot execute arbitrary shell commands or request arbitrary cloud URLs.

Claude's bridge wraps only an existing status-line command the user configured. Disconnect restores it only when the installed command still matches. Maxxit never executes suggested coding prompts automatically.
