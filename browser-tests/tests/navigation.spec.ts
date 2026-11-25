import { expect, test, type Page } from '@playwright/test';
import { documentLoadsFromNowOn } from './documentLoads';

const home = { path: '/', title: 'Home' };
const catalogs = { path: '/catalogs', title: 'Catalogs' };
const catalogCreated = { path: '/catalogs', title: 'Catalog created' };
const anItemsTab = /\/catalogs\/\d+\/items$/;
const aRemoval = /\/catalogs\/\d+\/remove$/;
const aSearch = /\/catalogs\?q=/;

async function expectViewShownOnceInTheRootOutlet(
  page: Page,
  { path, title }: { path: string; title: string },
) {
  await expect(page).toHaveURL(path);
  await expect(page).toHaveTitle(title);
  await expect(page.locator('main').getByRole('heading', { level: 1 })).toHaveText(title);
  await expect(page.getByRole('navigation')).toHaveCount(1);
  await expect(page.locator('main')).toHaveCount(1);
}

function answerToThePostedForm(page: Page) {
  return page.waitForResponse((response) => response.request().method() === 'POST');
}

const nameOfTheCatalogCreated = (page: Page) => page.locator('main strong').innerText();

async function startCatalogFromHome(page: Page) {
  await page.goto(home.path);
  await page.getByRole('button', { name: 'Start a catalog' }).click();
  await expectViewShownOnceInTheRootOutlet(page, catalogCreated);
  return nameOfTheCatalogCreated(page);
}

test('a click on a plain link swaps the outlet and updates the address bar', async ({
  page,
  javaScriptEnabled,
}) => {
  await page.goto(home.path);
  await expectViewShownOnceInTheRootOutlet(page, home);
  const loads = documentLoadsFromNowOn(page);

  await test.step('a link in the nested view, with no htmx attributes of its own', async () => {
    await page.getByRole('link', { name: 'See the catalogs' }).click();
    await expectViewShownOnceInTheRootOutlet(page, catalogs);
  });

  await test.step("a link in the root view's navigation", async () => {
    await page.getByRole('navigation').getByRole('link', { name: 'Home' }).click();
    await expectViewShownOnceInTheRootOutlet(page, home);
  });

  expect(loads).toHaveLength(javaScriptEnabled ? 0 : 2);
});

test('refresh, back and forward each show the view the address names', async ({ page }) => {
  await page.goto(home.path);
  await page.getByRole('link', { name: 'See the catalogs' }).click();
  await expectViewShownOnceInTheRootOutlet(page, catalogs);

  await page.reload();
  await expectViewShownOnceInTheRootOutlet(page, catalogs);

  await page.goBack();
  await expectViewShownOnceInTheRootOutlet(page, home);

  await page.goForward();
  await expectViewShownOnceInTheRootOutlet(page, catalogs);

  await page.goBack();
  await page.reload();
  await expectViewShownOnceInTheRootOutlet(page, home);
});

test('a posted form answered with 201 shows the answer, and the catalog is listed', async ({
  page,
  javaScriptEnabled,
}) => {
  await page.goto(home.path);
  const loads = documentLoadsFromNowOn(page);
  const answer = answerToThePostedForm(page);

  await page.getByRole('button', { name: 'Start a catalog' }).click();

  expect((await answer).status()).toBe(201);
  await test.step("the answer is shown at the address the form posted to, the collection's", async () => {
    await expectViewShownOnceInTheRootOutlet(page, catalogCreated);
    expect(loads).toHaveLength(javaScriptEnabled ? 0 : 1);
  });
  const name = await nameOfTheCatalogCreated(page);

  await test.step('the answer links to the catalog it created, at an address of its own', async () => {
    await page.getByRole('link', { name: `View ${name}`, exact: true }).click();
    await expect(page).toHaveURL(anItemsTab);
    await expect(page.locator('main').getByRole('heading', { level: 1 })).toHaveText(name);
  });

  await page.getByRole('link', { name: 'All catalogs' }).click();
  await expectViewShownOnceInTheRootOutlet(page, catalogs);
  await expect(page.getByRole('link', { name: `Remove ${name}`, exact: true })).toHaveCount(1);
});

test('a posted form answered with 303 ends on the view it redirects to, changed', async ({
  page,
  javaScriptEnabled,
}) => {
  const name = await startCatalogFromHome(page);
  const remove = page.getByRole('link', { name: `Remove ${name}`, exact: true });

  await test.step('the catalog is found through a search', async () => {
    await page.goto(home.path);
    await page.getByRole('link', { name: 'See the catalogs' }).click();
    await page.getByRole('searchbox', { name: 'Name' }).fill(name);
    await page.getByRole('button', { name: 'Search' }).click();
    await expect(page).toHaveURL(aSearch);
  });

  await test.step('removing is a view of its own, which asks first', async () => {
    await remove.click();
    await expect(page).toHaveURL(aRemoval);
    await expect(page).toHaveTitle('Remove catalog');
  });
  const loads = documentLoadsFromNowOn(page);
  const answer = answerToThePostedForm(page);

  await page.getByRole('button', { name: `Remove ${name}`, exact: true }).click();

  expect((await answer).status()).toBe(303);
  await expect(remove).toHaveCount(0);
  await test.step('the bar shows the view redirected to, not the address posted to', async () => {
    await expectViewShownOnceInTheRootOutlet(page, catalogs);
  });
  await test.step('without JavaScript the browser loads the answer and then follows it', async () => {
    expect(loads).toHaveLength(javaScriptEnabled ? 0 : 2);
  });

  await test.step('that address can be refreshed, and the catalog is still gone', async () => {
    await page.reload();
    await expectViewShownOnceInTheRootOutlet(page, catalogs);
    await expect(remove).toHaveCount(0);
  });

  await test.step("the form's answer is a step of its own in the history", async () => {
    await page.goBack();
    await expect(page).toHaveURL(aRemoval);
    await page.goBack();
    await expect(page).toHaveURL(aSearch);
    await page.goBack();
    await expectViewShownOnceInTheRootOutlet(page, catalogs);
    await page.goBack();
    await expectViewShownOnceInTheRootOutlet(page, home);
  });
});
