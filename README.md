
# Friendly Frame: Static Site Generator & Web Framework

A static site generator written in rust.
Intended to...
- Be beginner friendly
- Have the minimal amount of features to allow for static website development, whilst providing codebase structure, and minimumise repeated code.
- Have a folder structure and syntax that is obvious to understand for a new developer


# Setup
- [rust](https://rustup.rs)
- sass: `npm install -g sass`
- typescript: `npm install -g typescript` (only needed if you use .ts files)

(on debian / ubuntu, ./compile.sh offers to install anything missing for you)

# Quickstart / To run:
In order for the webpage to be viewed, the edit-me/ directory must be 'compiled'.
```bash
./compile.sh       # only compiles files changed since the last run
./compile.sh -f    # (or --fresh) deletes the output and compiles everything from scratch
./compile.sh -gp   # (or --github-pages) also removes '.html' from links
```
Upon compiling, the actual-website-do-not-edit/ directory will contain the final website code.

## Running locally (for testing)
```bash
./run-local.sh     # kills whatever is already using port 8000
```
then open http://localhost:8000 (the example website shows every example below, with the code that produced it)

---

# Folder layout
* edit-me/ = the code you edit (see it for a full example)
    * pages/ = one folder per page (e.g. pages/index/index.html -> index.html)
    * shared/ = things shared across multiple pages
* actual-website-do-not-edit/ = the compiled website
    * ! *do not modify the actual-website-do-not-edit/ directory manually* !

# Usage

## Reusable elements
==> If you have code that will be shared amongst multiple pages..
1) Create the reusable element in the 'edit-me/shared/reusables' folder
e.g. 'edit-me/shared/reusables/top-bar/'
```bash
top-bar.html   # only the body (no <html>, <head> etc.)
top-bar.scss   # optional, linked automatically
top-bar.ts     # optional, linked automatically
```
2) Add it to 'edit-me/shared/reusables-compilation-order.txt'
    - If your component uses other components, place it lower down than the components it uses.
3) Reuse that component in your other pages:
```html
<r-top-bar>
```

and if it requires additional parameters:
```html
<r-top-bar title="Friendly Frame" inner=@template:signpost subtitle=@none:>
<r-author picture="authors.jane_doe.svg">
```
then in top-bar.html / author.html:
```
{title}     -> Friendly Frame
{inner}     -> <r-signpost>    (which then gets compiled as well)
{subtitle}  ->                 (nothing)
[picture]   -> Jane Doe        (display name from the filename)
```

## Reusable typescript
In any .ts file, `<r-currentYear>` is replaced with edit-me/shared/code/currentYear.ts
(add `code` to the top of the compilation order file)

## Paths
- All paths are relative to the project root
* Add a project root placeholder using:
```html
'<root>/path/to/thing.txt'
```
e.g. to reference the file at: edit-me/shared/global.scss
```html
'<root>/shared/global.css'
```
- Works on any file type.

## Adding text (we use a special method since each page is translatable!)
```html
<div>TEXT=index.welcome</div>
```
is replaced with "welcome" from "index" in each edit-me/shared/page_text/*.json
--> one page per language: en-index.html, it-index.html...
--> index.html gets the default language (en)

## Blogs
Any page that ends up in actual-website-do-not-edit/blog/ (e.g. edit-me/pages/blogs/blog/first-post.html) only needs its content.
It gets wrapped in:
- edit-me/shared/reusables/template-blog-top/template-blog-top.html
- edit-me/shared/reusables/template-blog-bottom/template-blog-bottom.html
