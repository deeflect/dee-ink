# dee-porkbun

Full Porkbun API wrapper CLI for domains, DNS, DNSSEC, and SSL.

## Install

```sh
cargo install --path crates/dee-porkbun
```

## Quick start

```sh
dee-porkbun config set api_key pk1_xxx
dee-porkbun config set secret_key sk1_xxx
dee-porkbun domains pricing --tld com --json
dee-porkbun domains list-all --json
dee-porkbun dns retrieve dee.ink --json
dee-porkbun dns create dee.ink --type A --name www --content 1.1.1.1 --confirm --json
dee-porkbun dnssec get dee.ink --json
dee-porkbun ssl retrieve dee.ink --json
```

## Commands

- `config`
- `domains`
- `dns`
- `dnssec`
- `ssl`

## Safety

Mutating operations require explicit confirmation flags where applicable.

## Agent-friendly output

Use `--json` on all subcommands for machine output.

## Help

```sh
dee-porkbun --help
dee-porkbun <group> --help
dee-porkbun <group> <command> --help
```
