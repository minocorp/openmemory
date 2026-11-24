export const seedPages = [
  {
    path: "start",
    content: `Where things live.

- [[customers/acme]], [[customers/globex]] and [[customers/initech]] are clients
- [[projects/website-redesign]] and [[projects/mobile-app]] are active work
- meetings/ has one page per meeting, tax/ has filing dates

Agents: read this page first. Write each fact to the page it belongs to and add a timeline line with its source.
`,
  },
  {
    path: "customers/acme",
    content: `---
status: active
contact: Jane Park
fee: 500000
renewal: 2027-03-01
---
Acme runs the online store for a kitchenware brand. We maintain the storefront and the order admin.

## Terms

- Monthly retainer of 500,000 won, invoiced on the 1st
- Response within one business day
- [ ] Send the November invoice
- [x] Renew the SSL certificate

## Timeline
2026-01-15 | call — First call about a maintenance contract
2026-03-02 | contracts/acme.pdf#p1 "monthly fee of 500,000 won" — Contract signed
`,
  },
  {
    path: "customers/globex",
    content: `---
status: paused
contact: Tom Lee
fee: 300000
---
Globex paused the analytics dashboard until their Q1 budget is approved.

## Timeline
2026-08-04 | contracts/globex.pdf#p1 — Phase 1 signed
2026-09-10 | email — Paused until the Q1 budget review
`,
  },
  {
    path: "customers/initech",
    content: `---
status: prospect
contact: Bill Lumbergh
---
Initech wants an internal tool for stock counts. Budget around 8,000,000 won.

## Timeline
2026-11-18 | notes/initech-call.txt "Budget around 8,000,000 won" — Asked for a quote
`,
  },
  {
    path: "projects/website-redesign",
    content: `---
customer: acme
status: in progress
due: 2026-12-20
---
Redesign the Acme storefront with new product pages and a faster checkout.

## Scope

- Product page with a variant picker
- One-page checkout
- Lighthouse score above 90 on mobile

## Open questions

- Who writes the new product copy?
`,
  },
  {
    path: "projects/mobile-app",
    content: `---
customer: globex
status: planning
tags: [mobile, offline]
---
A field app for Globex sales staff to log store visits offline.

## Next

- Pick between React Native and a PWA
- Sketch the visit form
`,
  },
  {
    path: "meetings/2026-11-20-weekly",
    content: `---
date: 2026-11-20
---
- Website redesign is on track for 12-20
- Globex has not answered about the Q1 budget
- Quote for Initech goes out next week
`,
  },
  {
    path: "tax/2026-h2-vat",
    content: `---
due: 2027-01-25
status: todo
---
VAT filing for July to December. Collect the Acme and Globex invoices first.
`,
  },
  {
    path: "decisions/hosting",
    content: `We host every client project on one VPS with Docker Compose.

Why: one bill, one backup job, and the load is small.

Alternatives: a managed platform per client costs more, and Kubernetes is too much to run alone.
`,
  },
];
