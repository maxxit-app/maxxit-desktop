# Maxxit

Codex and Claude Code allowance, together on your Mac. Check remaining allowance in the menu bar, browse local observations, and keep a list of projects worth finishing.

Built with Tauri 2, React, and Rust. Local analytics are free and work without a Maxxit account. Maxxit Pro adds project ideas and reset emails for $9.99/month after service setup is complete.

![Maxxit overview with sample data](docs/screenshots/overview.jpg)

## Installation

macOS 14 or later, Apple silicon or Intel.

```sh
curl -fsSL https://raw.githubusercontent.com/maxxit-app/maxxit-desktop/main/scripts/install.sh | bash
```

Review [the installer](scripts/install.sh) before running it. It verifies the checksum and Apple signature of a published release. While the signed release is pending, it builds the tagged source on your Mac. That first build takes several minutes and needs Homebrew and Apple's command-line developer tools:

```sh
# Install Homebrew using the instructions at https://brew.sh
xcode-select --install
# Finish Apple's installation, then run the Maxxit command above.
```

The app installs into `~/Applications/Maxxit.app`. Node.js and Rust are only needed for the preview's local build. A signed binary release will not require them. No installation step disables Gatekeeper.

```sh
open "$HOME/Applications/Maxxit.app"
```

## Connect your assistants

1. Sign in through the official CLI: `codex login` or `claude`.
2. Open **Connections** in Maxxit.
3. Select **Connect local usage** for the provider you use. Claude shows the proposed status-line change before applying it; existing output is preserved.
4. Use the assistant. Maxxit refreshes once a minute, including while its window is closed.

![Connection controls](docs/screenshots/connections.jpg)

Click the menu bar icon for a compact usage panel. It shows every reported allowance window, reset countdowns, freshness, observed Codex token activity, and reset reminder status. Hover over a reset or chart bar for its exact time or token count. Click outside or press Escape to close the panel. You can also open it from View > Show usage panel with Cmd+Shift+U. Right-click the icon for the basic Open, Refresh, and Quit menu.

![Menu bar usage panel with sample data](docs/screenshots/tray.jpg)

These screenshots show the actual interface with labeled sample data. Direct third-party provider OAuth is not available. Maxxit does not read provider token files, browser sessions, or passwords. Codex usage comes from local session records; Claude allowance comes from its documented status line. Claude may omit allowance until its next API response.

Missing, expired, or observations older than two hours are shown as unavailable. Token charts contain observed local session increments and are explicitly partial. Percentages describe allowance windows, not billing credits or the cost of a task.

## Usage analytics

Choose 7, 14, or 30 days in Analytics. The chart shows a continuous UTC timeline with a token scale, period total, average per observed day, and busiest observed day. Select a bar with the mouse or keyboard for its exact total. Dots mark dates without records, which are excluded from the average. Today is still in progress. Export CSV saves the selected period with blank totals for unknown dates.

![Usage analytics with sample data](docs/screenshots/analytics.png)

## Optional cloud features

Connect a Maxxit account in **Connections**, approve the one-time code in the website, and choose usage and project sharing separately. The device credential stays in Mac Keychain. Usage sync sends allowance fields; project sharing sends only the descriptions you write. Conversation content, full paths, and provider credentials are excluded.

Pro checkout opens Polar in your browser. After paying, enable project sharing for ideas and verify your email in website settings for reset emails. Billing and email remain unavailable until the owner supplies credentials. Ideas never run automatically.

## Development

Use Node.js 24, pnpm 10.19.0, Rust, and Apple's command-line developer tools.

```sh
corepack enable
pnpm install --frozen-lockfile
pnpm desktop:dev
pnpm build
pnpm test
pnpm native:test
pnpm desktop:build
```

Use `pnpm dev` and open `http://127.0.0.1:1420/?demo` for the isolated browser preview. Demo data is never loaded in the native app.

## Uninstall

Disconnect Claude first to restore its previous status-line command. Disconnect your Maxxit account to revoke the device credential. Quit from the menu bar, then move `~/Applications/Maxxit.app` to Trash. Local data is in `~/Library/Application Support/app.maxxit.desktop`; delete that folder only if you want to remove local preferences, projects, and observations.

## Security and trademarks

Report vulnerabilities privately to contact@maxxit.app. See [SECURITY.md](SECURITY.md). [MIT license](LICENSE) applies to Maxxit code. Provider names and marks belong to their owners and do not imply endorsement; [asset attribution](public/providers/ATTRIBUTION.md). Manrope is licensed under the included SIL Open Font License.
