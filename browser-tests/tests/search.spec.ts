import { expect, test, type Page } from '@playwright/test';
import {
  autumnCoffees,
  catalogsTheDemoStartsWith,
  itemsPath,
  springTeas,
  summerTeas,
  winterCoffees,
} from './catalogs';
import { documentLoadsFromNowOn } from './documentLoads';

function listedCatalogNames(page: Page) {
  return page
    .locator('main')
    .getByRole('listitem')
    .getByRole('link')
    .filter({ hasNotText: /^Remove / });
}

type Search = { path: string; searched: string; found: string[] };

async function expectSearchWithExactlyItsResults(page: Page, { path, searched, found }: Search) {
  await expect(page).toHaveURL(path);
  await expect(listedCatalogNames(page)).toHaveText(found);
  await expect(page.getByRole('searchbox', { name: 'Name' })).toHaveValue(searched);
  await expect(page.locator('main')).toHaveCount(1);
}

// Other tests start and remove catalogs of their own, which are listed after these.
async function expectListBeginsWithTheCatalogsTheDemoStartsWith(page: Page) {
  for (const [position, catalog] of catalogsTheDemoStartsWith.entries()) {
    await expect(listedCatalogNames(page).nth(position)).toHaveText(new RegExp(`^${catalog.name}$`));
  }
}

async function search(page: Page) {
  await page.getByRole('button', { name: 'Search' }).click();
}

test('searching updates the address bar, and a refresh shows the same filtered results', async ({
  page,
  javaScriptEnabled,
}) => {
  const teas = { path: '/catalogs?q=tea', searched: 'tea', found: [springTeas.name, summerTeas.name] };
  await page.goto('/catalogs');
  await expectListBeginsWithTheCatalogsTheDemoStartsWith(page);
  const loads = documentLoadsFromNowOn(page);

  await page.getByRole('searchbox', { name: 'Name' }).fill('tea');
  await search(page);

  await expectSearchWithExactlyItsResults(page, teas);
  await test.step('htmx sends the form; without JavaScript the browser loads its answer', async () => {
    expect(loads).toHaveLength(javaScriptEnabled ? 0 : 1);
  });

  await page.reload();
  await expectSearchWithExactlyItsResults(page, teas);

  // The search box is not checked after going back: without JavaScript the
  // browser itself puts back what was typed in it.
  await test.step('back shows every catalog again', async () => {
    await page.goBack();
    await expect(page).toHaveURL('/catalogs');
    await expectListBeginsWithTheCatalogsTheDemoStartsWith(page);
  });
});

test('a search for what a URL cannot hold as it is comes back the same', async ({ page }) => {
  const nothing = { path: '/catalogs?q=r+t%26%3D%3F', searched: 'r t&=?', found: [] };
  await page.goto('/catalogs');

  await page.getByRole('searchbox', { name: 'Name' }).fill(nothing.searched);
  await search(page);

  await expectSearchWithExactlyItsResults(page, nothing);
  await page.reload();
  await expectSearchWithExactlyItsResults(page, nothing);
});

test('a result links to its resource, at an address that can be refreshed', async ({ page }) => {
  const coffees = {
    path: '/catalogs?q=coffee',
    searched: 'coffee',
    found: [autumnCoffees.name, winterCoffees.name],
  };
  await page.goto(coffees.path);
  await expectSearchWithExactlyItsResults(page, coffees);

  await page.getByRole('link', { name: winterCoffees.name, exact: true }).click();

  const heading = page.locator('main').getByRole('heading', { level: 1 });
  await expect(page).toHaveURL(itemsPath(winterCoffees));
  await expect(heading).toHaveText(winterCoffees.name);
  await page.reload();
  await expect(page).toHaveURL(itemsPath(winterCoffees));
  await expect(heading).toHaveText(winterCoffees.name);

  await page.goBack();
  await expectSearchWithExactlyItsResults(page, coffees);
});

test('checked tags are a list in the address, and a refresh shows the same results', async ({
  page,
}) => {
  const icedTeas = { path: '/catalogs?q=&tag=tea&tag=iced', searched: '', found: [summerTeas.name] };
  const iced = {
    path: '/catalogs?q=&tag=iced',
    searched: '',
    found: [summerTeas.name, autumnCoffees.name],
  };
  await page.goto('/catalogs');

  await page.getByRole('checkbox', { name: 'tea' }).check();
  await page.getByRole('checkbox', { name: 'iced' }).check();
  await search(page);

  await test.step("the address repeats the field's name, once for each tag", async () => {
    await expectSearchWithExactlyItsResults(page, icedTeas);
  });
  await page.reload();
  await expectSearchWithExactlyItsResults(page, icedTeas);
  await expect(page.getByRole('checkbox', { name: 'tea' })).toBeChecked();
  await expect(page.getByRole('checkbox', { name: 'iced' })).toBeChecked();
  await expect(page.getByRole('checkbox', { name: 'hot' })).not.toBeChecked();

  await test.step('one tag fewer is another address, with more catalogs', async () => {
    await page.getByRole('checkbox', { name: 'tea' }).uncheck();
    await search(page);
    await expectSearchWithExactlyItsResults(page, iced);
  });
});
