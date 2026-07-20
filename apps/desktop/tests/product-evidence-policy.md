# Product Evidence Policy

Status: active
Date: 2026-06-29

Critical product evidence in this directory must use the live local API started by Playwright.

Do not use `page.route`, `route.fulfill` or static success payloads for these flows:

- setup preview;
- setup accept;
- runtime install;
- runtime verify;
- model download;
- model verify;
- workspace open;
- agent prompt;
- file tree;
- terminal command execution.

Audit specs may observe blocked or incomplete behavior and write findings, but they must not convert a fake success response into product proof.
