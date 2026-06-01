---
layout: page
permalink: /students/
title: Students
description:
nav: true
nav_order: 3
---

<div class="publications">

Students I have mentored, grouped by institution. Degrees: PhD, MS (master's), UG (undergraduate).

{% for group in site.data.students %}
  <h2 class="bibliography">
    {% if group.url != "" %}<a href="{{ group.url }}">{{ group.institution }}</a>{% else %}{{ group.institution }}{% endif %}
  </h2>
  <ol class="bibliography">
  {% for m in group.members %}
    <li>
      <div class="row">
        <div class="col col-sm-2 abbr">
          <abbr class="badge rounded w-100">{{ m.degree }}</abbr>
        </div>
        <div class="col-sm-10">
          <div class="title">{{ m.name }}</div>
          <div class="author">{{ m.topic }}</div>
          {% if m.destination != "" %}
          <div class="periodical">&rarr; {{ m.destination }}</div>
          {% endif %}
        </div>
      </div>
    </li>
  {% endfor %}
  </ol>
{% endfor %}

</div>
