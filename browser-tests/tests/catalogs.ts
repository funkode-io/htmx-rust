export type Catalog = { id: number; name: string; tags: string[]; items: string[] };

export const springTeas: Catalog = {
  id: 1,
  name: 'Spring teas',
  tags: ['tea', 'hot'],
  items: ['Sencha', 'Darjeeling'],
};
export const summerTeas: Catalog = {
  id: 2,
  name: 'Summer teas',
  tags: ['tea', 'iced'],
  items: ['Iced oolong', 'Cold brew green'],
};
export const autumnCoffees: Catalog = {
  id: 3,
  name: 'Autumn coffees',
  tags: ['coffee', 'iced'],
  items: ['Cold brew', 'Iced latte'],
};
export const winterCoffees: Catalog = {
  id: 4,
  name: 'Winter coffees',
  tags: ['coffee', 'hot'],
  items: ['Espresso', 'Flat white'],
};

export const catalogsTheDemoStartsWith = [springTeas, summerTeas, autumnCoffees, winterCoffees];

export const tagsPath = (catalog: Catalog) => `/catalogs/${catalog.id}/tags`;
export const itemsPath = (catalog: Catalog) => `/catalogs/${catalog.id}/items`;
export const itemPath = (catalog: Catalog, item: string) =>
  `${itemsPath(catalog)}/${encodeURIComponent(item)}`;
