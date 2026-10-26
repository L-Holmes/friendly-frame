
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
==> If you have code that will be shared amongst multiple pages..
#### 1) Create the reusable element in the 'edit-me/shared/reusables' folder
e.g. 'edit-me/shared/reusables/top-bar', with all of the usual files that a regular page would have:
```bash
top-bar.html
top-bar.scss   # (optional) compiled + linked automatically
top-bar.ts     # (optional) compiled + linked automatically
```
But! The html file would only be the body:
```html
<div class="top-bar"> Hi, this is the top-bar! </div>
```

#### 2) Add it to 'edit-me/shared/reusables-compilation-order.txt'
- Components are compiled in the order they are listed (lines that start with '#' are ignored)
- If your component uses other components, place it lower down than the components it uses.

#### 3) Reuse that component in your other pages
```html
<r-top-bar>
```

and if it requires additional parameters:
```
<r-top-bar title="Friendly Frame" inner=@template:signpost subtitle=@none:>

# where top-bar.html is:
<div class="top-bar">
	<div class="top-bar-title">{title}</div>
	<div class="top-bar-subtitle">{subtitle}</div>
	{inner}
</div>

vvvvvvvvvvvvvvvvvvvvvv

# produces:
<div class="top-bar">
	<div class="top-bar-title">Friendly Frame</div>
	<div class="top-bar-subtitle"></div>
	<div class="signpost"> ...the compiled signpost component... </div>
</div>
```
`[param]` gives a display name from a filename instead, e.g. `picture="authors.jane_doe.svg"` -> `[picture]` = Jane Doe

## Reusable code
In any .ts file, `<r-filename>` is replaced with edit-me/shared/code/filename.ts
(add `code` to the top of the compilation order file)
```ts
//The below will be replaced with edit-me/shared/code/currentYear.ts once the website is compiled
<r-currentYear>
```

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
- File types this works on:
    - any.

## Adding text (we use a special method since each page is translatable!)
1) Add the text to each language file in edit-me/shared/page_text/ (en.json, it.json...)
```json
{ "index": { "welcome": "Welcome!" } }
```
2) .. and then for each div / heading etc. in which you want text:
```html
<div>TEXT=index.welcome</div>
```
--> each page is compiled once per language: en-index.html, it-index.html...
--> index.html gets the default language (en)

## Blogs
Any page that ends up in actual-website-do-not-edit/blog/ only needs its content.
It gets wrapped in the 'template-blog-top' and 'template-blog-bottom' reusable elements.


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
