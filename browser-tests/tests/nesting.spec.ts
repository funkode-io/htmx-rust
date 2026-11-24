import { expect, test, type Page } from '@playwright/test';

// A catalog's view holds nested views with addresses of their own: its tabs,
// the tags and the items, and inside the items the one item the address names.
// Each address shows its view inside every view around it, however the reader
// got there. Each scenario runs with and without JavaScript.

/** The catalog's view is shown, once, in the root view's outlet, with its tabs. */
function catalog(page: Page, name: string) {
  const outlet = page.locator('#outlet-0');
  return {
    outlet,
    async expectShown() {
      await expect(outlet).toHaveCount(1);
      await expect(outlet.getByRole('heading', { level: 1 })).toHaveText(name);
      await expect(outlet.getByRole('navigation').getByRole('link')).toHaveText(['Items', 'Tags']);
    },
  };
}

/** Of the catalog's tabs, `open` is the one marked as open, and the only one. */
async function expectOpenTab(page: Page, open: string) {
  const marked = page.locator('#outlet-1 > nav [aria-current]');
  await expect(marked).toHaveText([open]);
}

/** The tags of "Spring teas" are shown inside its view. */
async function expectTags(page: Page) {
  const { outlet, expectShown } = catalog(page, 'Spring teas');
  await expect(page).toHaveURL('/catalogs/1/tags');
  await expect(page).toHaveTitle('Tags');
  await expectShown();
  await expect(outlet.getByRole('heading', { level: 2 })).toHaveText('Tags');
  await expect(outlet.locator('section').getByRole('listitem')).toHaveText(['tea', 'hot']);
  await expectOpenTab(page, 'Tags');
}

/** The items of "Summer teas" are shown inside its view, and inside them `item`, if any. */
async function expectItems(page: Page, path: string, title: string, item?: string) {
  const { outlet, expectShown } = catalog(page, 'Summer teas');
  const items = outlet.locator('section');
  await expect(page).toHaveURL(path);
  await expect(page).toHaveTitle(title);
  await expectShown();
  await expect(items.getByRole('heading', { level: 2 })).toHaveText('Items');
  await expect(items.getByRole('listitem')).toHaveText(['Iced oolong', 'Cold brew green']);
  // The item is below the list, in the outlet of the items.
  const shown = items.locator('ul + div').getByRole('heading', { level: 3 });
  if (item) await expect(shown).toHaveText(item);
  else await expect(shown).toHaveCount(0);
  // Of the items listed, the one that is open is marked, and only it.
  await expect(items.locator('li [aria-current]')).toHaveText(item ? [item] : []);
  await expectOpenTab(page, 'Items');
}

test('a nested view opened at its address, and refreshed, is shown inside its parent', async ({
  page,
}) => {
  await page.goto('/catalogs/1/tags');
  await expectTags(page);

  await page.reload();
  await expectTags(page);
});

test('a view four views deep opened at its address, and refreshed, is shown inside every view around it', async ({
  page,
}) => {
  await page.goto('/catalogs/2/items/Iced%20oolong');
  await expectItems(page, '/catalogs/2/items/Iced%20oolong', 'Item', 'Iced oolong');

  await page.reload();
  await expectItems(page, '/catalogs/2/items/Iced%20oolong', 'Item', 'Iced oolong');
});

test('links between nested views change the address and what is shown inside the catalog', async ({
  page,
  javaScriptEnabled,
}) => {
  await page.goto('/catalogs/2');
  await catalog(page, 'Summer teas').expectShown();
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(0);
  const loads: string[] = [];
  page.on('request', (request) => {
    if (request.resourceType() === 'document') loads.push(request.url());
  });

  await page.getByRole('link', { name: 'Items' }).click();
  await expectItems(page, '/catalogs/2/items', 'Items');

  await page.getByRole('link', { name: 'Cold brew green' }).click();
  await expectItems(page, '/catalogs/2/items/Cold%20brew%20green', 'Item', 'Cold brew green');

  // Another item takes the place of the first.
  await page.getByRole('link', { name: 'Iced oolong' }).click();
  await expectItems(page, '/catalogs/2/items/Iced%20oolong', 'Item', 'Iced oolong');

  // htmx fetches and swaps; without JavaScript each click loads a document.
  expect(loads).toHaveLength(javaScriptEnabled ? 0 : 3);

  await page.goBack();
  await expectItems(page, '/catalogs/2/items/Cold%20brew%20green', 'Item', 'Cold brew green');

  // The item links to the view around it: the same items, with none shown.
  await page.getByRole('link', { name: 'Close' }).click();
  await expectItems(page, '/catalogs/2/items', 'Items');

  // The tags take the place of the items, in the same catalog.
  await page.getByRole('link', { name: 'Tags' }).click();
  await expect(page).toHaveURL('/catalogs/2/tags');
  await catalog(page, 'Summer teas').expectShown();
  await expect(page.getByRole('heading', { level: 2 })).toHaveText('Tags');
  await expect(page.getByRole('heading', { level: 3 })).toHaveCount(0);
});

test('a tab click keeps the catalog around the tabs as it was, and the address bar shows the tab', async ({
  page,
  javaScriptEnabled,
}) => {
  await page.goto('/catalogs/1');
  const header = page.locator('#outlet-0').getByRole('heading', { level: 1 });
  await expect(header).toHaveText('Spring teas');
  // A property of the node, which no answer can carry: only the node itself keeps it.
  await header.evaluate((node) => Object.assign(node, { kept: true }));

  await page.getByRole('link', { name: 'Tags' }).click();
  await expectTags(page);

  // With htmx the header is the same DOM node; without it, one of a new document.
  expect(await header.evaluate((node) => 'kept' in node)).toBe(javaScriptEnabled);
});

test('back and forward across tab clicks show the tab each address names', async ({ page }) => {
  const summer = catalog(page, 'Summer teas');
  /** The tags of "Summer teas", or with `open` false the catalog with no tab open. */
  const expectTab = async (path: string, title: string, open: boolean) => {
    await expect(page).toHaveURL(path);
    await expect(page).toHaveTitle(title);
    await summer.expectShown();
    await expect(page.getByRole('navigation')).toHaveCount(2);
    await expect(page.getByRole('heading', { level: 2 })).toHaveText(open ? ['Tags'] : []);
    await expect(page.locator('#outlet-1 > nav [aria-current]')).toHaveText(open ? ['Tags'] : []);
  };
  await page.goto('/catalogs/2');
  await expectTab('/catalogs/2', 'Catalog', false);
  await page.getByRole('link', { name: 'Items' }).click();
  await expectItems(page, '/catalogs/2/items', 'Items');
  await page.getByRole('link', { name: 'Tags' }).click();
  await expectTab('/catalogs/2/tags', 'Tags', true);

  await page.goBack();
  await expectItems(page, '/catalogs/2/items', 'Items');
  await page.goBack();
  await expectTab('/catalogs/2', 'Catalog', false);

  await page.goForward();
  await expectItems(page, '/catalogs/2/items', 'Items');
  await page.goForward();
  await expectTab('/catalogs/2/tags', 'Tags', true);
});
