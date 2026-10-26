"use strict";
function currentYear() {
    return new Date().getFullYear().toString();
}
const year = document.querySelector(".index-year");
if (year)
    year.textContent = currentYear();
