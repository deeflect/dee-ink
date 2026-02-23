# dee-porkbun — Agent Guide

Full Porkbun API CLI wrapper.

## Install
```bash
cargo install dee-porkbun
```

## Setup
1. Enable API access in Porkbun account settings.
2. Copy `API Key` and `Secret API Key`.
3. Save keys:
```bash
dee-porkbun config set api_key <API_KEY>
dee-porkbun config set secret_key <SECRET_API_KEY>
```
4. Verify key presence:
```bash
dee-porkbun config show --json
```

## Quick Start
```bash
dee-porkbun domains ping --json
dee-porkbun domains list-all --json
dee-porkbun dns retrieve example.com --json
```

## Command groups
- `config`: set/show/path
- `domains`: ping, pricing, list-all, check, create, update-ns, get-ns, update-auto-renew, add/get/delete URL forwarding, create/update/delete/get glue
- `dns`: create/edit/delete/retrieve by id and by name/type
- `dnssec`: create/get/delete
- `ssl`: retrieve

## Safety
Mutating commands require `--confirm`:
- domain create/update operations
- URL forward add/delete
- glue create/update/delete
- DNS create/edit/delete
- DNSSEC create/delete

Missing confirm response:
```json
{"ok":false,"error":"Confirmation required: rerun with --confirm","code":"CONFIRM_REQUIRED"}
```

## Output contract
- List success:
```json
{"ok":true,"count":2,"items":[...]}
```
- Item success:
```json
{"ok":true,"item":{...}}
```
- Action success:
```json
{"ok":true,"message":"DNS record updated"}
```
- Error:
```json
{"ok":false,"error":"...","code":"API_ERROR"}
```

## Common workflows
### Workflow: Check and register a domain
```bash
dee-porkbun domains check mybrand.com --json
dee-porkbun domains create mybrand.com --cost 1108 --agree-to-terms --confirm --json
```

### Workflow: DNS management
```bash
dee-porkbun dns retrieve mydomain.com --json
dee-porkbun dns create mydomain.com --type A --name www --content 1.1.1.1 --ttl 600 --confirm --json
dee-porkbun dns edit-by-name-type mydomain.com A www --content 1.1.1.2 --confirm --json
```

### Workflow: Nameservers and URL forwarding
```bash
dee-porkbun domains update-ns mydomain.com --ns ns1.example.com --ns ns2.example.com --confirm --json
dee-porkbun domains add-url-forward mydomain.com --subdomain blog --location https://blog.example.com --type temporary --include-path no --wildcard no --confirm --json
dee-porkbun domains get-url-forwarding mydomain.com --json
```

### Workflow: DNSSEC and SSL bundle
```bash
dee-porkbun dnssec get mydomain.com --json
dee-porkbun ssl retrieve mydomain.com --json
```

## Storage
- Config: `~/.config/dee-porkbun/config.toml`
- Data: none

## Exit codes
- `0` success
- `1` error
