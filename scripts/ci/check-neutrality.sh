#!/usr/bin/env bash
# Fail if a forbidden vendor/product name appears in a tracked PUBLIC artifact (ADR-0003).
# This deny-list is a regression backstop, not the rule: review is authoritative and the list
# is non-exhaustive. Gitignored internal working state is outside tracked-file `git grep`.
# This script excludes ITSELF (its own name list would self-match) and honors the one
# carve-out: GOVERNANCE.md MAY name the first implementer per AGENTS.md/ADR-0003. Prior-art
# hyperlinks are permitted and are not listed here.
set -uo pipefail

self='scripts/ci/check-neutrality.sh'
found=0

# Forbidden everywhere (no carve-out). `-w` guards word boundaries against substring hits.
# AI/LLM vendors and products:
if git grep -nwEi 'LlamaIndex|LangChain|LangGraph|OpenAI|ChatGPT|Anthropic|Gemini|Copilot' -- ":!$self"; then found=1; fi
# Vector stores, search engines, and data platforms:
if git grep -nwEi 'Pinecone|Weaviate|Qdrant|Elasticsearch|OpenSearch|Databricks|Snowflake|Salesforce' -- ":!$self"; then found=1; fi
# Collaboration and tracking tools whose names are not English words:
if git grep -nwEi 'Jira' -- ":!$self"; then found=1; fi
# Names that collide with common English words are matched case-sensitively:
if git grep -nwE 'Notion|Confluence|Slack|Chroma' -- ":!$self"; then found=1; fi
# The first-implementer name is carved out for GOVERNANCE.md only:
if git grep -nEi 'Nurok' -- ":!GOVERNANCE.md" ":!$self"; then found=1; fi

if [ "$found" -ne 0 ]; then
  echo "::error::forbidden vendor/product name found in a public artifact (ADR-0003); see matches above."
  exit 1
fi
echo "Neutrality OK: no forbidden vendor names in public artifacts."
