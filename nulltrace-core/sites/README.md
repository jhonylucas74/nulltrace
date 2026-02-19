# Site VMs

This folder defines **site/server VMs** that are created (or reset) on every cluster startup. Each subfolder represents one site; the folder name should match the site address (e.g. `ntml.org`).

- **Haru** and other player-owned VMs are unchanged: they are created once if missing and then restored from the database. Only site VMs are driven by this folder.

## Structure

```
sites/
  <dns_name>/           e.g. ntml.org
    config.yaml         VM config (hostname, cpu, memory, bootstrap, etc.)
    www/                optional: files to copy into the VM at /var/www
      index.ntml
      404.ntml
      ...
```

## config.yaml

- `hostname`: VM hostname (e.g. `ntml-server`)
- `dns_name`: optional; defaults to the folder name (e.g. `ntml.org`)
- `cpu_cores`, `memory_mb`, `disk_mb`: VM resources
- `bootstrap`: string written to `/etc/bootstrap` and run on VM start (e.g. `httpd /var/www`)

All site VMs are owned by the **webserver** player. On startup, for each site folder that has a `config.yaml`, the cluster will destroy any existing VM for that site (same owner + dns_name) and create a new one from the config, then seed `/var/www` from the folder’s `www/` if present.
