---
layout: page
permalink: /publications/
title: Publications
description:
nav: true
nav_order: 2
---

<!-- _pages/publications.md -->

<!-- Bibsearch Feature -->

{% include bib_search.liquid %}

<div class="publications">

The full list of publications can be found on <a href="https://scholar.google.com/citations?hl=en&user=lqirTjkAAAAJ&view_op=list_works&sortby=pubdate">Google Scholar</a>.

<h2 class="bibliography">RL Safety</h2>
{% bibliography --group_by none --query @*[category=rl]* %}

<h2 class="bibliography">LLM Safety</h2>
{% bibliography --group_by none --query @*[category=llm]* %}

<h2 class="bibliography">Agent Safety</h2>
{% bibliography --group_by none --query @*[category=agent]* %}

</div>
