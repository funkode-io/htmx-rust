# Forms use GET and POST, and answer as HTTP says

A form is the hypermedia control for everything a link cannot do. GET is for safe requests: a search or filter form is a link whose query string comes from its fields. POST is for everything unsafe, sent to a resource's path, where a POST controller answers it, so a form cannot post to a route that does not exist ([ADR 6](0006-a-resource-answers-through-a-controller-and-a-presenter.md)). Its presenter answers with HTTP's own meanings: `201 Created` with a `Location` header and a body that links to the new resource, `303 See Other` to a view, or a `4xx` with the form, its values and its errors (`409` when the command refuses, `422` when the input is invalid). Whatever HTML it answers with is drawn in the place of the view that holds the form ([ADR 4](0004-failures-are-drawn-in-place.md)), so the result is the same with and without JavaScript.

## Considered Options

- **PUT, PATCH and DELETE through htmx**: HTML forms cannot send them, so those actions would stop working without JavaScript.
- **Always redirect after a change**: one rule, and a refresh never re-sends the form, but it rules out the plain `201` answer and needs a place outside the URL to carry "created" to the next page. The app chooses per action.

## Consequences

- Protection against forged posts (CSRF) and sign-in are the app's, through axum layers; the core has no notion of a session.
