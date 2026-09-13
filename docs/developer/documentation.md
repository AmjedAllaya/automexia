# Developer documentation guide

Public documentation describes the free baseline terminal, its current behavior,
and the maintenance contracts needed to build, test, secure, package, and use
it.

## Before editing

Read [Documentation policy](../DOCUMENTATION.md),
[Private documentation policy](../PRIVATE-DOCUMENTATION-POLICY.md), the canonical
page for the topic, and the current source/tests that establish behavior.

## Public writing

State only behavior documented in [Features](../FEATURES.md). Separate shipped
behavior from exact external evidence still required for that same public
baseline. Do not publish feature teasers, private names, schemas, algorithms,
workflows, integrations, provider matrices, pricing, market analysis, or
commercial packaging.

## Verification

Load every Markdown file, check UTF-8 and code fences, resolve relative file and
heading links, scan for confidential or machine-local values, scan for private
plan language, inspect the complete diff, and verify the ignored private archive
remains intact.

A source identifier or test fixture is not authorization to announce a product.
