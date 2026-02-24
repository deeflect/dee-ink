# AGENT — dee-food

## Purpose
Search restaurants and fetch details/reviews from Yelp.

## Typical flow
1. `dee-food config set yelp.api-key <KEY>`
2. `dee-food search "Austin, TX" --term bbq --json`
3. `dee-food show <business-id> --json`
4. `dee-food reviews <business-id> --json`
