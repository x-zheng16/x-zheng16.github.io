// Small enhancements for the homepage; every page reads fine without them.
(function () {
  var nav = document.querySelector(".topnav");
  if (nav) {
    var onScroll = function () { nav.classList.toggle("scrolled", window.scrollY > 8); };
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
  }

  // "and N more" on long author lists
  document.querySelectorAll(".au-b .morebtn").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var au = btn.closest(".au");
      au.querySelector(".au-x").hidden = false;
      btn.parentNode.remove();
    });
  });

  // "upcoming" tag on talks whose date (YYYY-MM-DD, or YYYY-MM for the whole month) has not passed
  var now = new Date();
  document.querySelectorAll(".tag[data-until]").forEach(function (tag) {
    var p = tag.getAttribute("data-until").split("-").map(Number);
    var end = p.length > 2 ? new Date(p[0], p[1] - 1, p[2] + 1) : new Date(p[0], p[1], 1);
    if (now < end) tag.hidden = false;
  });
})();
