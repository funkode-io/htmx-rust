import { type Page } from '@playwright/test';

export function documentLoadsFromNowOn(page: Page) {
  const loads: string[] = [];
  page.on('request', (request) => {
    if (request.resourceType() === 'document') loads.push(request.url());
  });
  return loads;
}
