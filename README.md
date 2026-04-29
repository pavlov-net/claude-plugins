# stuart-plugins

A personal [Claude Code](https://docs.claude.com/en/docs/claude-code) plugin marketplace — small, focused plugins for development workflows and productivity.

## Install

Add the marketplace, then install the plugins you want:

```
/plugin marketplace add pavlov-net/claude-plugins
/plugin install <plugin-name>@stuart-plugins
```

## Plugins

| Plugin | Description |
| --- | --- |
| [`auto-format`](./auto-format) | Auto-format files after Claude edits them. Supports Go (goimports/gofmt), Rust (rustfmt), Python (ruff/black), and JS/TS (biome/prettier). |
| [`efficient-commands`](./efficient-commands) | Teaches Claude to use shell commands efficiently — avoid re-running expensive commands and stop tail/head chasing. |
| [`tsgo-lsp`](./tsgo-lsp) | TypeScript 7 native LSP — uses [tsgo](https://www.npmjs.com/package/@typescript/native-preview) for faster type checking and diagnostics. |
| [`bevy`](./bevy) | Authoritative idioms for [Bevy 0.18](https://bevyengine.org/) game projects in Rust — ECS, communication, scheduling, plugins, assets, UI, errors, testing, performance. |

## License

MIT
