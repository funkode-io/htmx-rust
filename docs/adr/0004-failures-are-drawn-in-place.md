# A failure is an outcome, drawn in the place of the view it replaces

A view cannot fail ([ADR 6](0006-a-resource-answers-through-a-controller-and-a-presenter.md)). When a use case answers with an error (the catalog does not exist), the presenter of that resource answers with an outcome like any other: a status and a view, such as `404` and "there is no such catalog". That view is shown where the resource's own view would have been: the views above it stay around it on a full load, and htmx 4 swaps it into the same outlet on a click. Nothing nested below it is asked. A presenter may instead answer with no view, for cases such as a `303` to sign-in, and that is then the whole response.

An empty result is not a failure: a search that finds nothing answers `200` and the view draws "nothing found" as ordinary content.

## Considered Options

- **A generic error page**: loses the navigation around the failure, and shows a different thing on a click than on a refresh.
- **A view returns a result** (`render` answers `Result`): the view would choose between drawing and failing, which is the presenter's choice, and it would have to load data to know.
- **The presenter answers a status and a URL, and the view at that URL replaces the page** (`/not-found`): the address the reader asked for is lost.
