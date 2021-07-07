# chia-harvester-metrics

Collects and exports useful metrics from the [chia-blockchain](https://github.com/Chia-Network/chia-blockchain) harvester.

## Metrics Exported

- `chia_log_lines` [`level`] - The total number of log lines processed, with a 'level' label (INFO, WARNING, ERROR, etc).
- `chia_harvester_events_total` - The total number of harvester events processed.
- `chia_harvester_plots_total` - The total number of active plots the harvester is harvesting.
- `chia_harvester_plots_eligible` - The total number of harvester plots found eligible.
- `chia_harvester_plots_proofs` - The total number of proofs found 😁

## Running

You must pass the path to Chia's `debug.log` file, which will need to be enabled in the chia-blockchain config first. You may also pass an optional socket address and port the metrics server will listen on, by default we listen on all interfaces on port 4041.

```
chia-harvester-metrics --log-file /home/ubuntu/.chia/mainnet/log/debug.log --listen-addr 0.0.0.0:4041
```

It's best to run the metrics server in a service or something that will maintain itself through system restarts.

## Monitoring

You can use these metrics to display some useful info on, for example, a Grafana dashboard! Here's an example:

![dashboard](https://media.discordapp.net/attachments/854064320404127776/862167579187609600/unknown.png?width=1100&height=660)

This project works great in combination with [node_exporter](https://github.com/prometheus/node_exporter) for more detailed disk/cpu/network statistics on a machine. Go wild!
