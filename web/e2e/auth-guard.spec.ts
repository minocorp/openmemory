import { expect, test } from "@playwright/test";

import { shoot } from "./support/screenshot";

test("sends a signed-out visitor to the sign-in screen", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/login$/);
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await shoot(page, "01-login");
});

test("returns to the page that was asked for after signing in", async ({ page }) => {
  await page.goto("/p/customers/Missing");
  await expect(page).toHaveURL(/\/login\?next=%2Fp%2Fcustomers%2FMissing$/);
});

test("shows the not-found page for an unknown route", async ({ page }) => {
  await page.goto("/missing");
  await expect(page.getByRole("heading", { name: "Page not found" })).toBeVisible();
});

test("explains a bootstrap link without a token", async ({ page }) => {
  await page.goto("/bootstrap");
  await expect(page.getByRole("heading", { name: "This link is incomplete" })).toBeVisible();
});
