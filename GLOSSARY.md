# htmx-rust

A library for server-rendered Rust web apps enhanced with htmx, where the URL alone decides what the browser shows.

## Language

**Resource**:
What a URL names: a type whose fields are what its own part of the URL carries, the parameters of its part of the path and its fields of the query string. It holds nothing of the resources around it. A resource may be nested inside another, to any depth; the root resource, nested in no other, is at `/`.
_Avoid_: Route, endpoint, page, address

**Resource path**:
Where a resource stands (the type `ResourcePath`): its value and, above it, the path of the resource it is nested in, up to the root resource. It prints as the URL that names the resource there, and that URL is read back into the same path. A GET on it answers with that resource again, in the representation the request asks for. Every link and every `Location` is a resource path; no URL is written by hand.
_Avoid_: Route, link builder, view path, URL for the value. "Path" for a resource's own part of it alone (its "part of the path")

**Controller**:
What answers one method on a resource: it receives the request and calls one use case of the application, a read model for GET and a command for POST.
_Avoid_: Handler, action for the code

**Presenter**:
What turns a use case's result into an outcome.
_Avoid_: Responder, mapper

**Outcome**:
What a presenter answers with: a status, a `Location` when the status needs one, and a view when there is something to show.
_Avoid_: Response for the presenter's answer: the response is what the library sends

**View**:
What is drawn: a value that holds everything it shows, and a function from that value to HTML. It does not wait, cannot fail, and knows nothing of the request, of the use case or of the views around it. A presenter chooses it. The view of the root resource is the whole document.
_Avoid_: Template, screen, a view that loads. Page and section for views by their depth: they are all views. Layout for the root resource's view: a layout is how a view arranges its space (columns, a grid, a collapsible panel), which is the app's own markup

**Outlet**:
The one place in a view where the view of the resource nested in its own is shown. The URL decides what fills it, not the code that uses the view, which is what sets it apart from a slot.
_Avoid_: Slot, frame, placeholder

**Action**:
An unsafe request a form sends: a change to a resource, such as creating or archiving a catalog.
_Avoid_: Mutation, handler. Command for the request: a command is what the action's controller calls

**Region**:
A part of the interface that is refreshed on its own, without changing the path. Its state lives in the query string, and it can be reused in any view.
_Avoid_: Fragment, partial, widget
