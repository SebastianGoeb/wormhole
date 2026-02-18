import { expect, test } from "@playwright/test";

test("value is synchronized", async ({ context }) => {
  const page1 = await context.newPage();
  const page2 = await context.newPage();

  await page1.goto("http://localhost:3000/");
  await page2.goto("http://localhost:3000/");

  await expect(page1).toHaveTitle("Wormhole");
  await expect(page2).toHaveTitle("Wormhole");

  await page1.locator("#main-input").fill("123")
  await expect(page2.locator("#main-input")).toHaveValue("123");

  const ttl = 5000;
  await expect(page1.locator("#main-input")).toHaveValue("", {timeout: ttl + 1000});
  await expect(page2.locator("#main-input")).toHaveValue("");
});
