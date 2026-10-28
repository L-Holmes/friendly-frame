
# Friendly Frame: Static Site Generator & Web Framework

A static site generator written in rust.
Intended to...
- Be beginner friendly
- Have the minimal amount of features to allow for static website development, whilst providing codebase structure, and minimumise repeated code.
- Have a folder structure and syntax that is obvious to understand for a new developer


# Quickstart

## Making the website
In order for the webpage to be viewed, the edit-me/ directory must be 'compiled'.
This is done by running the compile script;

```bash
./compile.sh       # only compiles files changed since the last run
./compile.sh -f    # (or --fresh) compiles everything from scratch
./compile.sh -gp   # (or --github-pages) also removes '.html' from links
```

Upon compiling, the actual-website-do-not-edit/ directory will contain the final website code.
(needs [rust](https://rustup.rs), sass and typescript. On debian / ubuntu, ./compile.sh installs anything missing for you)

## Running locally (for testing)
```bash
./run-local.sh     # serves actual-website-do-not-edit/ at http://localhost:8000
```
The example website in edit-me/ shows every feature below.


# Coding guidance

## Reusable elements
**You write:**

`edit-me/pages/index/index.html`
```html
<r-top-bar title="Friendly Frame" inner=@template:signpost subtitle=@none:>
...
<r-author picture="authors.jane_doe.svg">
```
`edit-me/shared/reusables/top-bar/top-bar.html`
```html
<div class="top-bar">
    <div class="top-bar-title">{title}</div>
    <div class="top-bar-subtitle">{subtitle}</div>
    {inner}
</div>
```
`edit-me/shared/reusables/author/author.html`
```html
<div class="author">
    <img class="author-picture" src="<root>/shared/images/{picture}" alt="">
    [picture]
</div>
```
`edit-me/shared/reusables-compilation-order.txt`
```
code
reusables/signpost
reusables/author
reusables/top-bar
```
**Compiles into:**

`actual-website-do-not-edit/index.html`
```html
<div class="top-bar">
    <div class="top-bar-title">Friendly Frame</div>
    <div class="top-bar-subtitle"></div>
    <div class="signpost">...</div>
</div>
...
<div class="author">
    <img class="author-picture" src="shared/images/authors.jane_doe.svg" alt="">
    Jane Doe
</div>
```
Write something once in edit-me/shared/reusables/ (its .scss and .ts get linked automatically), add it to the compilation order below anything it uses, then use it on any page with one tag.
`{x}` becomes the value you give it, `[x]` a name made from that file name, `@template:x` another reusable element, and `@none:` nothing.

## Reusable typescript
**You write:**

`edit-me/pages/index/index.ts`
```ts
<r-currentYear>

const year = document.querySelector(".index-year");
if (year) year.textContent = currentYear();
```
`edit-me/shared/code/currentYear.ts`
```ts
function currentYear(): string {
    return new Date().getFullYear().toString();
}
```
**Compiles into:**

`actual-website-do-not-edit/index.js`
```js
"use strict";
function currentYear() {
    return new Date().getFullYear().toString();
}
const year = document.querySelector(".index-year");
if (year)
    year.textContent = currentYear();
```
In any .ts file, `<r-currentYear>` is replaced with edit-me/shared/code/currentYear.ts (add `code` to the top of the compilation order file).

## Paths
**You write:**

`edit-me/pages/index/index.html`
```html
<link rel="stylesheet" href="<root>/shared/global.css">
```
`edit-me/shared/reusables/template-blog-top/template-blog-top.html`
```html
<link rel="stylesheet" href="<root>/shared/global.css">
```
**Compiles into:**

`actual-website-do-not-edit/index.html`
```html
<link rel="stylesheet" href="shared/global.css">
```
`actual-website-do-not-edit/blog/first-post.html`
```html
<link rel="stylesheet" href="../shared/global.css">
```
`<root>` becomes the path back to the root of the website, from wherever the file ends up. Works on any file type.

## Adding text
**You write:**

`edit-me/pages/index/index.html`
```html
<div class="index-title"><div>TEXT=index.welcome</div></div>
```
`edit-me/shared/page_text/en.json`
```json
{ "index": { "welcome": "Welcome!", ... }, ... }
```
`edit-me/shared/page_text/it.json`
```json
{ "index": { "welcome": "Benvenuto!", ... }, ... }
```
**Compiles into:**

`actual-website-do-not-edit/index.html` + `en-index.html`
```html
<div class="index-title"><div>Welcome!</div></div>
```
`actual-website-do-not-edit/it-index.html`
```html
<div class="index-title"><div>Benvenuto!</div></div>
```
Text goes in edit-me/shared/page_text/, one file per language. Each page gets compiled once per language, and index.html gets the default language (en).

## Blogs
**You write:**

`edit-me/pages/blogs/blog/first-post.html`
```html
<div class="blog-title"><div>TEXT=blog.first_title</div></div>
<r-author picture="authors.jane_doe.svg">
<div>TEXT=blog.first_text</div>
```
`edit-me/shared/reusables/template-blog-top/template-blog-top.html`
```html
<!DOCTYPE html>
<html lang="en">
    <head>...</head>
    <body>
        <r-top-bar title="Friendly Frame" inner=@template:signpost subtitle="Blog">
        <div class="page-content">
```
`edit-me/shared/reusables/template-blog-bottom/template-blog-bottom.html`
```html
            <a href="<root>/index.html"><div>TEXT=blog.back</div></a>
        </div>
    </body>
</html>
```
**Compiles into:**

`actual-website-do-not-edit/blog/first-post.html`
```html
<!DOCTYPE html>
<html lang="en">
    <head>...</head>
    <body>
        <div class="top-bar">...</div>
        <div class="page-content">
            <div class="blog-title"><div>My first post</div></div>
            <div class="author">...</div>
            <div>This file only has a title, an author and this sentence.</div>
            <a href="../index.html"><div>← Back home</div></a>
        </div>
    </body>
</html>
```
Any page that ends up in actual-website-do-not-edit/blog/ only needs its content: the rest of the page comes from the two blog templates.


# Technology used
* rust = the compiler
* html / typescript
* scss = improved css for styling
    * compiled via the compilation script


# Folder layout
* 'edit-me/' contains the code that you edit
* 'actual-website-do-not-edit/' contains the compiled code that represents the source code of the final website
    * ! *do not modify the actual-website-do-not-edit/ directory manually* !

### --> edit-me/pages
Contains things that are specific to single pages (and the pages themselves)
* defines the routes for a website (i.e. at which url each asset is found)
* defines the static content unique to each page (in the html file)
* defines the dynamic content unique to each page (in the ts file)

### --> edit-me/shared
Contains things that are shared across multiple pages / the entire website
* edit-me/shared/reusables = the reused html components (e.g. top bar, since it is used in many pages) + their scss/ts
* edit-me/shared/images = images
* edit-me/shared/page_text = page text / translations
* edit-me/shared/code = typescript that is reused amongst different pages
* edit-me/shared/global.scss = styling that is used across all pages


# Aims
* Should be straight forward to use if you only have experience with html/css/js
    * Should compile directly to that
    * Should have a minimal number of files that are not directly related to html/css/js
* Should understand everything just from this read me
* Folder structure is intuitive whilst organised
* Minimal number of dependencies
* You can literally read/modify the compiler yourself if there are any issues

## Differences when compared to other static-site-generators:
* No need for anything to be installed (apart from rust, sass and typescript)
* Easy
* Doesn't focus on templating with markdown files


# Limitations / Rules
* Can't have text in your page that is of the form: <r-nameOfAReusableComponent>
* Reusable components must be located inside of the directory: edit-me/shared/reusables/
* Anything that ends up in actual-website-do-not-edit/blog/ is treated as a blog
* The compiler must be run next to the edit-me/ and actual-website-do-not-edit/ directories (./compile.sh does this for you)


# Coding guidelines / recommendations
* Grid should be used most of the time (flex only to align things left / centre / right)
    * Why? simplicity, easier to change + reliable.
* Use a minimal number of html tag types (div for pretty much everything)
    * Why? Minimises the number of things to learn, less confusion.
* Everything has a class. No #Name or anything like that
    * Should see the html, look at the class, then go to the class and be able to change *all* the styling.
* In 99% of cases, don't use javascript to add repeated text to the page.
    * Use reusable elements to make the compiler generate the html instead
    * Why? typically faster, and easier to interpret as the compiled html will look similar to the actual page
