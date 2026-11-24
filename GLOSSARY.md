# htmx-rust

A library for server-rendered Rust web apps enhanced with htmx, where the URL alone decides what the browser shows.

## Language

**View**:
What one URL shows: the path names a resource and the view is its representation, complete on its own. A view's fields are what its own part of the URL carries, the parameters of its part of the path and its fields of the query string; it holds nothing of the views around it. A view may be nested inside another view, to any depth; the root view, nested in no other, is the whole document.
_Avoid_: Route, screen, handler. Page and section for views by their depth: they are all views. Layout for the root view: a layout is how a view arranges its space (columns, a grid, a collapsible panel), which is the app's own markup

**View path**:
Where a view is shown (the type `ViewPath`): a view value and, above it, the path of the view it is nested in, up to the root view. It prints as the URL that shows the view there, and that URL is read back into the same path. A view renders at its path, from which it reaches its ancestors' values, and the paths of the views beside it and nested in it. Every link is a path; no URL is written by hand.
_Avoid_: Route, link builder, URL for the value. "Path" for a view's own part of it alone (its "part of the path")

**Outlet**:
The one place in a view where the view nested inside it is shown. The URL decides what fills it, not the code that uses the view, which is what sets it apart from a slot. An outlet has an id, by which a request from htmx names it: the answer is then cut to what goes inside that outlet, and the views around it do not render.
_Avoid_: Slot, frame, placeholder

**Action**:
An unsafe request a form sends: a change to a resource, such as creating or archiving a catalog.
_Avoid_: Mutation, command, handler

**Region**:
A part of the interface that is refreshed on its own, without changing the path. Its state lives in the query string, and it can be reused in any view.
_Avoid_: Fragment, partial, widget
