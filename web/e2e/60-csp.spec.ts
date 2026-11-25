import { expect, test } from "@playwright/test";

import { openAdminPage } from "./support/admin-session";
import { apiSend } from "./support/api";
import { embedded } from "./support/config";
import { simplePdf } from "./support/pdf";

test.describe("content security policy", () => {
  test.skip(
    !embedded,
    "The policy comes from the server; run with E2E_EMBEDDED=1 on a built web/dist",
  );

  test("the main screens run without a policy violation", async ({ browser }) => {
    const { page } = await openAdminPage(browser);
    const violations: string[] = [];
    await page.exposeFunction("reportViolation", (description: string) => {
      violations.push(description);
    });
    await page.addInitScript(() => {
      document.addEventListener("securitypolicyviolation", (event) => {
        const report = Reflect.get(window, "reportViolation");
        report(`${event.violatedDirective} ${event.blockedURI} ${event.sourceFile}`);
      });
    });
    await apiSend(page, {
      method: "PUT",
      path: "/api/pages?path=csp-e2e/Page",
      headers: { "If-None-Match": "*" },
      json: { content: "---\nstatus: active\n---\nBody with **bold** text.\n" },
    });
    await apiSend(page, {
      method: "POST",
      path: "/api/files?path=csp-e2e/doc.pdf",
      contentType: "application/pdf",
      body: simplePdf([["Policy check", "Monthly fee of 500,000 won"]]),
    });

    const response = await page.goto("/p/csp-e2e/Page?file=csp-e2e%2Fdoc.pdf");
    expect(response?.headers()["content-security-policy"]).toContain("frame-ancestors 'none'");
    await expect(page.getByRole("region", { name: "File csp-e2e/doc.pdf" })).toBeVisible();
    await expect(page.locator(".textLayer span").first()).toBeVisible();
    await expect(page.locator(".cm-content")).toBeVisible();
    await apiSend(page, {
      method: "POST",
      path: "/api/files?path=csp-e2e/mockup.html",
      contentType: "text/html",
      body: Buffer.from("<h1>Framed mockup</h1>"),
    });
    await page.goto("/p/csp-e2e/Page?file=csp-e2e%2Fmockup.html");
    await expect(
      page
        .frameLocator('iframe[title="mockup.html"]')
        .getByRole("heading", { name: "Framed mockup" }),
    ).toBeVisible();
    for (const path of ["/", "/settings", "/settings?tab=people", "/settings?tab=system"]) {
      await page.goto(path);
      await page.waitForLoadState("networkidle");
    }
    expect(violations).toEqual([]);
    await page.context().close();
  });
});
