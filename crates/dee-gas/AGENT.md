# AGENT — dee-gas

## Purpose
Fetch US average gasoline price data from EIA in machine-friendly format.

## Typical flow
1. `dee-gas config set eia.api-key <KEY>`
2. `dee-gas national --json`
3. `dee-gas prices --state CA --json`
4. `dee-gas history --state TX --weeks 8 --json`
