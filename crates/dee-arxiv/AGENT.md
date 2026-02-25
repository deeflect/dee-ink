# AGENT — dee-arxiv

## Purpose
Query academic papers with stable machine output for agents.

## Typical flow
1. `dee-arxiv search "rust async" --limit 5 --json`
2. `dee-arxiv search "llm reasoning" --sort citations --json`
3. `dee-arxiv get 2312.12345 --json`
4. `dee-arxiv author "Yann LeCun" --limit 10 --json`
