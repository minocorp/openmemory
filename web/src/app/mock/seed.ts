import { appendTimeline, createPage, editPage } from "./page-writes";
import { simplePdf } from "./pdf";
import { seedPages } from "./seed-pages";
import { current, nextId, pageAt, state } from "./store";

const minute = 60_000;
const ago = (minutes: number) => new Date(Date.now() - minutes * minute).toISOString();
const day = 24 * 60;
const today = () => new Date().toISOString().slice(0, 10);

export function seed() {
  seedAccounts();
  seedFiles();
  for (const [index, entry] of seedPages.entries()) {
    createPage(entry.path, entry.content, { author: "mino", at: ago(20 * day - index * 60) });
  }
  state.lastSeenSeq = state.seq;
  seedActivity();
}

function seedAccounts() {
  state.users.push(
    { id: 1, username: "mino", display_name: "Mino", is_admin: true, created_at: ago(90 * day) },
    {
      id: 2,
      username: "sam",
      display_name: "Sam Choi",
      is_admin: false,
      created_at: ago(30 * day),
    },
  );
  state.passkeys.push(
    { id: 1, name: "MacBook Touch ID", created_at: ago(90 * day), last_used_at: ago(30) },
    { id: 2, name: "iPhone", created_at: ago(60 * day), last_used_at: ago(3 * day) },
  );
  const connection = (name: string, owner: string, scopes: string[], canWrite: boolean) => ({
    id: nextId(),
    name,
    owner,
    owner_display_name: owner === "mino" ? "Mino" : "Sam Choi",
    user_id: owner === "mino" ? 1 : 2,
    scopes,
    can_write: canWrite,
    created_at: ago(40 * day),
    last_used_at: ago(45),
  });
  state.connections.push(
    connection("claude-code", "mino", ["*"], true),
    connection("cursor", "mino", ["projects"], true),
    connection("codex", "mino", ["*"], false),
    connection("claude-desktop", "sam", ["customers", "projects"], true),
  );
  state.invites.push({
    id: nextId(),
    inviter: "mino",
    inviter_display_name: "Mino",
    created_at: ago(2 * day),
    expires_at: new Date(Date.now() + 5 * day * minute).toISOString(),
  });
}

function seedFiles() {
  const file = (path: string, mime: string, bytes: Uint8Array<ArrayBuffer>, pageCount: number) => ({
    id: nextId(),
    path,
    mime,
    bytes,
    pageCount,
    createdAt: ago(25 * day),
  });
  state.files.push(
    file(
      "contracts/acme.pdf",
      "application/pdf",
      simplePdf([
        ["Service agreement", "Acme Inc. and Mino", "Article 3 monthly fee of 500,000 won"],
        ["Amendment 1", "Monthly fee of 450,000 won from 2026-12-01"],
      ]),
      2,
    ),
    file(
      "contracts/globex.pdf",
      "application/pdf",
      simplePdf([["Statement of work", "Analytics dashboard, phase 1", "Fee 300,000 won"]]),
      1,
    ),
    file(
      "notes/initech-call.txt",
      "text/plain",
      new TextEncoder().encode(
        "Call with Initech, 2026-11-18\n\nThey want an internal tool for stock counts.\nBudget around 8,000,000 won.\nDecision by mid December.\n",
      ),
      1,
    ),
  );
}

function seedActivity() {
  editAt("customers/acme", "mino/claude-code", "Update the Acme contract", 95, (content) =>
    content
      .replace("Monthly retainer of 500,000 won", "Monthly retainer of 450,000 won from December")
      .replace("fee: 500000", "fee: 450000"),
  );
  const acme = pageAt("customers/acme");
  if (acme) {
    appendTimeline(
      acme,
      {
        text: "Agreed to lower the fee",
        source: 'contracts/acme.pdf#p2 "450,000 won"',
        date: today(),
      },
      { author: "mino/claude-code", task: "Update the Acme contract", at: ago(94) },
    );
  }
  createPage(
    "meetings/2026-11-24-acme-kickoff",
    "---\ndate: 2026-11-24\ncustomer: acme\n---\n## Decisions\n\n- Checkout ships first, product pages after\n- Jane reviews copy every Friday\n",
    { author: "mino/claude-code", task: "Write kickoff notes", at: ago(60) },
  );
  editAt("projects/website-redesign", "mino/cursor", "Sprint notes", 32, (content) =>
    content.replace(
      "- Who writes the new product copy?",
      "- Who writes the new product copy? Acme writes it, we edit.",
    ),
  );
  editAt("projects/mobile-app", "mino/cursor", "Sprint notes", 31, (content) =>
    content.replace("status: planning", "status: in progress"),
  );
  createPage(
    "customers/hooli",
    "---\nstatus: prospect\ncontact: Gavin Belson\n---\nMet at the Pangyo meetup. Wants a quote for a booking site.\n",
    { author: "sam", at: ago(12) },
  );
}

function editAt(
  path: string,
  author: string,
  task: string,
  minutesAgo: number,
  change: (content: string) => string,
) {
  const page = pageAt(path);
  if (page) {
    editPage(page, change(current(page).content), { author, task, at: ago(minutesAgo) });
  }
}
