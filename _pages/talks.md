---
layout: page
permalink: /talks/
title: Talks
description:
nav: true
nav_order: 3
---

<div class="publications">

{% assign years = site.data.talks | map: "year" | uniq | sort | reverse %}
{% for yr in years %}
  <h2 class="bibliography">{{ yr }}</h2>
  {% assign yr_talks = site.data.talks | where: "year", yr %}
  <ol class="bibliography">
  {% for talk in yr_talks %}
    <li>
      <div class="row">
        <div class="col col-sm-2 abbr">
          {% assign venue = site.data.venues[talk.venue_abbr] %}
          {% if venue and venue.url %}
            <abbr class="badge rounded w-100"><a href="{{ venue.url }}">{{ talk.venue_abbr }}</a></abbr>
          {% else %}
            <abbr class="badge rounded w-100">{{ talk.venue_abbr }}</abbr>
          {% endif %}
        </div>
        <div class="col-sm-10">
          <div class="title">{{ talk.title }}</div>
          <div class="author">{{ talk.host }}</div>
          <div class="periodical">
            {{ talk.location }}{% if talk.date != "" %} &middot; {{ talk.date }} {{ yr }}{% endif %}
          </div>
        </div>
      </div>
    </li>
  {% endfor %}
  </ol>
{% endfor %}

</div>
