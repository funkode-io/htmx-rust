import { expect, test, type Page } from '@playwright/test';

// What the server sends. A page that is loaded or refreshed is the whole
// document. A click with JavaScript gets only what goes inside the outlet the
// link is aimed at; without JavaScript every click loads the whole document.

/** The body of the answer to the GET request for `path` that `action` causes. */
async function answerTo(page: Page, path: string, action: () => Promise<unknown>) {
  const answer = page.waitForResponse(
    (response) =>
      response.request().method() === 'GET' && new URL(response.url()).pathname === path,
  );
  await action();
  return (await answer).text();
}

/** What only the root view draws: the document and the navigation of the app. */
const document = ['<!DOCTYPE html>', '<html', '<head>', '>Home</a>'];
/** What only the catalog's view draws, around its tabs: its header. */
const catalog = '<h1>Autumn coffees</h1>';
/** What the tabs' view draws around the tab that is open. */
const tabs = '>Items</a>';

function expectContains(html: string, parts: string[], contained: boolean) {
  for (const part of parts) {
    if (contained) expect(html).toContain(part);
    else expect(html).not.toContain(part);
  }
}

test('a load and a refresh get the whole document, a click only what goes inside the root outlet', async ({
  page,
  javaScriptEnabled,
}) => {
  const loaded = await answerTo(page, '/', () => page.goto('/'));
  expectContains(loaded, [...document, '<h1>Home</h1>'], true);

  const clicked = await answerTo(page, '/catalogs/3', async () => {
    await page.getByRole('link', { name: 'See the catalogs' }).click();
    await page.getByRole('link', { name: 'Autumn coffees' }).click();
  });
  // The catalog and its tabs either way; the document around them only without JavaScript.
  expectContains(clicked, [catalog, tabs], true);
  expectContains(clicked, document, !javaScriptEnabled);
  await expect(page).toHaveTitle('Catalog');

  const refreshed = await answerTo(page, '/catalogs/3', () => page.reload());
  expectContains(refreshed, [...document, catalog, tabs], true);
  await expect(page).toHaveTitle('Catalog');
});

test('a tab click gets the tabs and the tab, without the catalog around them', async ({
  page,
  javaScriptEnabled,
}) => {
  await page.goto('/catalogs/3');
  const tags = '<h2>Tags</h2>';

  const clicked = await answerTo(page, '/catalogs/3/tags', () =>
    page.getByRole('link', { name: 'Tags' }).click(),
  );
  expectContains(clicked, [tabs, tags], true);
  expectContains(clicked, [...document, catalog], !javaScriptEnabled);
  await expect(page.locator('#outlet-1 > nav [aria-current]')).toHaveText(['Tags']);

  const refreshed = await answerTo(page, '/catalogs/3/tags', () => page.reload());
  expectContains(refreshed, [...document, catalog, tabs, tags], true);

  // Back is answered with the whole document: htmx asks for all of it.
  const restored = await answerTo(page, '/catalogs/3', () => page.goBack());
  expectContains(restored, [...document, catalog, tabs], true);
  await expect(page.getByRole('heading', { level: 2 })).toHaveCount(0);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Autumn coffees');
});
