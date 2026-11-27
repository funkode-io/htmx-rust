import { expect, test, type Page } from '@playwright/test';
import { type Catalog, itemPath, itemsPath, springTeas, summerTeas, tagsPath } from './catalogs';
import { documentLoadsFromNowOn } from './documentLoads';

const catalogOutlet = (page: Page) => page.locator('main');
const tabOutlet = (page: Page) => catalogOutlet(page).locator('section');
const itemOutlet = (page: Page) => tabOutlet(page).locator('ul + div');

async function expectCatalogShownOnceWithItsTabs(page: Page, catalog: Catalog) {
  await expect(catalogOutlet(page)).toHaveCount(1);
  await expect(catalogOutlet(page).getByRole('heading', { level: 1 })).toHaveText(catalog.name);
  await expect(catalogOutlet(page).getByRole('navigation').getByRole('link')).toHaveText([
    'Items',
    'Tags',
  ]);
}

async function expectOnlyOpenTab(page: Page, open: string) {
  await expect(page.locator('main nav [aria-current]')).toHaveText([open]);
}

async function expectTagsTabInsideItsCatalog(page: Page, catalog: Catalog) {
  await expect(page).toHaveURL(tagsPath(catalog));
  await expect(page).toHaveTitle('Tags');
  await expectCatalogShownOnceWithItsTabs(page, catalog);
  await expect(tabOutlet(page).getByRole('heading', { level: 2 })).toHaveText('Tags');
  await expect(tabOutlet(page).getByRole('listitem')).toHaveText(catalog.tags);
  await expect(itemOutlet(page)).toHaveCount(0);
  await expectOnlyOpenTab(page, 'Tags');
}

async function expectItemsTabInsideItsCatalog(page: Page, catalog: Catalog) {
  await expectCatalogShownOnceWithItsTabs(page, catalog);
  await expect(tabOutlet(page).getByRole('heading', { level: 2 })).toHaveText('Items');
  await expect(tabOutlet(page).getByRole('listitem')).toHaveText(catalog.items);
  await expectOnlyOpenTab(page, 'Items');
}

async function expectItemsWithNoneOpen(page: Page, catalog: Catalog) {
  await expect(page).toHaveURL(itemsPath(catalog));
  await expect(page).toHaveTitle('Items');
  await expectItemsTabInsideItsCatalog(page, catalog);
  await expect(itemOutlet(page).getByRole('heading', { level: 3 })).toHaveCount(0);
}

async function expectItemOpenBelowTheItems(page: Page, catalog: Catalog, item: string) {
  await expect(page).toHaveURL(itemPath(catalog, item));
  await expect(page).toHaveTitle('Item');
  await expectItemsTabInsideItsCatalog(page, catalog);
  await expect(itemOutlet(page).getByRole('heading', { level: 3 })).toHaveText(item);
}

test('a nested view opened at its address, and refreshed, is shown inside its parent', async ({
  page,
}) => {
  await page.goto(tagsPath(springTeas));
  await expectTagsTabInsideItsCatalog(page, springTeas);

  await page.reload();
  await expectTagsTabInsideItsCatalog(page, springTeas);
});

test('a view three views deep opened at its address, and refreshed, is shown inside every view around it', async ({
  page,
}) => {
  const [item] = summerTeas.items;
  await page.goto(itemPath(summerTeas, item));
  await expectItemOpenBelowTheItems(page, summerTeas, item);

  await page.reload();
  await expectItemOpenBelowTheItems(page, summerTeas, item);
});

test('links between nested views change the address and what is shown inside the catalog', async ({
  page,
  javaScriptEnabled,
}) => {
  const [firstItem, secondItem] = summerTeas.items;

  await test.step("a catalog's own address opens its first tab, at that tab's address", async () => {
    await page.goto(`/catalogs/${summerTeas.id}`);
    await expectItemsWithNoneOpen(page, summerTeas);
  });
  const loads = documentLoadsFromNowOn(page);

  await test.step('an item opens below the items, and another takes its place', async () => {
    await page.getByRole('link', { name: secondItem }).click();
    await expectItemOpenBelowTheItems(page, summerTeas, secondItem);

    await page.getByRole('link', { name: firstItem }).click();
    await expectItemOpenBelowTheItems(page, summerTeas, firstItem);
  });

  await test.step('htmx swaps both; without JavaScript each click loads a document', async () => {
    expect(loads).toHaveLength(javaScriptEnabled ? 0 : 2);
  });

  await test.step('back shows the item opened before', async () => {
    await page.goBack();
    await expectItemOpenBelowTheItems(page, summerTeas, secondItem);
  });

  await test.step('an item links to the items around it, with none open', async () => {
    await page.getByRole('link', { name: 'Close' }).click();
    await expectItemsWithNoneOpen(page, summerTeas);
  });

  await test.step('each tab takes the place of the other, in the same catalog', async () => {
    await page.getByRole('link', { name: 'Tags' }).click();
    await expectTagsTabInsideItsCatalog(page, summerTeas);

    await page.getByRole('link', { name: 'Items' }).click();
    await expectItemsWithNoneOpen(page, summerTeas);
  });
});
